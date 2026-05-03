use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 服务工厂函数类型：每次调用产生一个新实例
type Factory = Arc<dyn Fn(&Container) -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// 单例存储：惰性初始化的服务实例
struct SingletonEntry {
    factory: Factory,
    instance: OnceLock<Arc<dyn Any + Send + Sync>>,
}

enum Binding {
    /// 每次 make 都调用工厂函数
    Factory(Factory),
    /// 全局单例，第一次 make 时初始化
    Singleton(Arc<SingletonEntry>),
    /// 直接注册的已有实例
    Instance(Arc<dyn Any + Send + Sync>),
}

/// IoC 服务容器
///
/// 支持三种绑定方式：
/// - `bind`：每次解析都调用工厂函数创建新实例
/// - `singleton`：全局单例，惰性初始化
/// - `instance`：直接注册已有实例
pub struct Container {
    bindings: Mutex<HashMap<TypeId, Binding>>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            bindings: Mutex::new(HashMap::new()),
        }
    }

    /// 绑定工厂函数，每次 make 创建新实例
    pub fn bind<T, F>(&self, factory: F)
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&Container) -> Arc<T> + Send + Sync + 'static,
    {
        let wrapped: Factory = Arc::new(move |c| {
            let svc: Arc<T> = factory(c);
            svc as Arc<dyn Any + Send + Sync>
        });
        let mut map = self.bindings.lock().unwrap();
        map.insert(TypeId::of::<T>(), Binding::Factory(wrapped));
    }

    /// 绑定单例工厂，全局只创建一次
    pub fn singleton<T, F>(&self, factory: F)
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&Container) -> Arc<T> + Send + Sync + 'static,
    {
        let wrapped: Factory = Arc::new(move |c| {
            let svc: Arc<T> = factory(c);
            svc as Arc<dyn Any + Send + Sync>
        });
        let entry = Arc::new(SingletonEntry {
            factory: wrapped,
            instance: OnceLock::new(),
        });
        let mut map = self.bindings.lock().unwrap();
        map.insert(TypeId::of::<T>(), Binding::Singleton(entry));
    }

    /// 直接注册已有实例（始终返回同一实例）
    pub fn instance<T>(&self, instance: Arc<T>)
    where
        T: Any + Send + Sync + 'static,
    {
        let boxed: Arc<dyn Any + Send + Sync> = instance;
        let mut map = self.bindings.lock().unwrap();
        map.insert(TypeId::of::<T>(), Binding::Instance(boxed));
    }

    /// 解析服务，返回 `Arc<T>`
    pub fn make<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let binding = {
            let map = self.bindings.lock().unwrap();
            map.get(&TypeId::of::<T>()).cloned().map(|b| match b {
                Binding::Factory(f) => BindingRef::Factory(f),
                Binding::Singleton(s) => BindingRef::Singleton(s),
                Binding::Instance(i) => BindingRef::Instance(i),
            })
        };

        match binding? {
            BindingRef::Factory(factory) => {
                let any = factory(self);
                any.downcast::<T>().ok()
            }
            BindingRef::Singleton(entry) => {
                let any = entry.instance.get_or_init(|| (entry.factory)(self));
                any.clone().downcast::<T>().ok()
            }
            BindingRef::Instance(inst) => inst.downcast::<T>().ok(),
        }
    }

    /// 检查服务是否已绑定
    pub fn has<T: Any + 'static>(&self) -> bool {
        let map = self.bindings.lock().unwrap();
        map.contains_key(&TypeId::of::<T>())
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

// 用于内部临时持有绑定的枚举（避免持有锁时调用工厂）
enum BindingRef {
    Factory(Factory),
    Singleton(Arc<SingletonEntry>),
    Instance(Arc<dyn Any + Send + Sync>),
}

// SingletonEntry 需要实现 Clone（通过 Arc 包装已满足）
impl Clone for Binding {
    fn clone(&self) -> Self {
        match self {
            Binding::Factory(f) => Binding::Factory(f.clone()),
            Binding::Singleton(s) => Binding::Singleton(s.clone()),
            Binding::Instance(i) => Binding::Instance(i.clone()),
        }
    }
}
