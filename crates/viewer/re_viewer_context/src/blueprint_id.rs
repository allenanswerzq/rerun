use std::hash::BuildHasher as _;

use re_log_types::{EntityPath, EntityPathPart};

pub trait BlueprintIdRegistry {
    fn registry_name() -> &'static str;
    fn registry_path() -> &'static EntityPath;
}

/// A unique id for a type of Blueprint contents.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
pub struct BlueprintId<T: BlueprintIdRegistry> {
    id: uuid::Uuid,
    #[serde(skip)]
    _registry: std::marker::PhantomData<T>,
}

/// Serde adapter for representing a [`BlueprintId`] as a UUID string.
///
/// Use `#[serde(with = "re_viewer_context::blueprint_id_serde")]` on a field.
/// This works for both [`ViewId`] and [`ContainerId`] without changing their default serialization.
pub mod blueprint_id_serde {
    use serde::{Deserialize as _, Deserializer, Serializer};

    use super::{BlueprintId, BlueprintIdRegistry};

    /// Serialize a blueprint ID as a UUID string.
    pub fn serialize<S, T>(id: &BlueprintId<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: BlueprintIdRegistry,
    {
        serializer.serialize_str(&id.uuid().to_string())
    }

    /// Deserialize a UUID string into a blueprint ID.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<BlueprintId<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: BlueprintIdRegistry,
    {
        let id = String::deserialize(deserializer)?;
        uuid::Uuid::try_parse(&id)
            .map_err(serde::de::Error::custom)
            .map(BlueprintId::from)
    }
}

impl<T: BlueprintIdRegistry> re_byte_size::SizeBytes for BlueprintId<T> {
    const IS_POD: bool = true;

    fn heap_size_bytes(&self) -> u64 {
        0
    }
}

impl<T: BlueprintIdRegistry> BlueprintId<T> {
    pub fn invalid() -> Self {
        Self {
            id: uuid::Uuid::nil(),
            _registry: std::marker::PhantomData,
        }
    }

    pub fn random() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            _registry: std::marker::PhantomData,
        }
    }

    pub const fn from_bytes(bytes: uuid::Bytes) -> Self {
        Self {
            id: uuid::Uuid::from_bytes(bytes),
            _registry: std::marker::PhantomData,
        }
    }

    pub fn from_entity_path(path: &EntityPath) -> Self {
        if !path.is_child_of(T::registry_path()) {
            return Self::invalid();
        }

        path.last()
            .and_then(|last| uuid::Uuid::parse_str(last.unescaped_str()).ok())
            .map_or_else(Self::invalid, |id| Self {
                id,
                _registry: std::marker::PhantomData,
            })
    }

    pub fn hashed_from_str(s: &str) -> Self {
        use std::hash::{Hash as _, Hasher as _};

        let salt1: u64 = 0x307b_e149_0a3a_5552;
        let salt2: u64 = 0x6651_522f_f510_13a4;

        let hash1 = {
            let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
            salt1.hash(&mut hasher);
            s.hash(&mut hasher);
            hasher.finish()
        };

        let hash2 = {
            let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
            salt2.hash(&mut hasher);
            s.hash(&mut hasher);
            hasher.finish()
        };

        let uuid = uuid::Uuid::from_u64_pair(hash1, hash2);

        uuid.into()
    }

    pub fn gpu_readback_id(self) -> re_renderer::GpuReadbackIdentifier {
        re_log_types::hash::Hash64::hash(self.id).hash64()
    }

    #[inline]
    pub fn as_entity_path(&self) -> EntityPath {
        std::iter::chain(
            T::registry_path().iter().cloned(),
            std::iter::once(EntityPathPart::new(self.id.to_string())),
        )
        .collect()
    }

    #[inline]
    pub fn registry() -> &'static EntityPath {
        T::registry_path()
    }

    #[inline]
    pub fn uuid(&self) -> uuid::Uuid {
        self.id
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        re_log_types::hash::Hash64::hash(self.id).hash64()
    }
}

impl<T: BlueprintIdRegistry> From<uuid::Uuid> for BlueprintId<T> {
    #[inline]
    fn from(id: uuid::Uuid) -> Self {
        Self {
            id,
            _registry: std::marker::PhantomData,
        }
    }
}

impl<T: BlueprintIdRegistry> From<re_sdk_types::encodings::Uuid> for BlueprintId<T> {
    #[inline]
    fn from(id: re_sdk_types::encodings::Uuid) -> Self {
        Self {
            id: id.into(),
            _registry: std::marker::PhantomData,
        }
    }
}

impl<T: BlueprintIdRegistry> From<BlueprintId<T>> for re_sdk_types::encodings::Uuid {
    #[inline]
    fn from(id: BlueprintId<T>) -> Self {
        id.id.into()
    }
}

impl<T: BlueprintIdRegistry> std::fmt::Display for BlueprintId<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", T::registry_name(), self.id.simple())
    }
}

impl<T: BlueprintIdRegistry> std::fmt::Debug for BlueprintId<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", T::registry_name(), self.id.simple())
    }
}

// ----------------------------------------------------------------------------
/// Helper to define a new [`BlueprintId`] type.
macro_rules! define_blueprint_id_type {
    ($type:ident, $registry:ident, $registry_name:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $registry;

        impl $registry {
            const REGISTRY: &'static str = $registry_name;
        }

        impl BlueprintIdRegistry for $registry {
            fn registry_name() -> &'static str {
                stringify!($type)
            }

            fn registry_path() -> &'static EntityPath {
                static REGISTRY_PATH: std::sync::LazyLock<EntityPath> =
                    std::sync::LazyLock::new(|| $registry::REGISTRY.into());
                &REGISTRY_PATH
            }
        }

        pub type $type = BlueprintId<$registry>;
    };
}

// ----------------------------------------------------------------------------
// Definitions for the different [`BlueprintId`] types.
define_blueprint_id_type!(ViewId, ViewIdRegistry, "view");
define_blueprint_id_type!(ContainerId, ContainerIdRegistry, "container");

impl ViewId {
    /// Returns the renderer identity corresponding to this blueprint view.
    pub fn render_view_id(self) -> re_renderer::ViewBuilderId {
        re_renderer::ViewBuilderId::new(re_log_types::hash::Hash64::hash(self.id).hash64())
    }
}

// ----------------------------------------------------------------------------
// Builtin `ViewId`s.

/// A dummy view for shared blueprint data between views.
///
/// This is currently not exposed for any api to interact with, but there is technically nothing
/// stopping us from manually adding it.
pub const GLOBAL_VIEW_ID: ViewId = ViewId::from_bytes([
    0x5C, 0x0D, 0xCA, 0x6A, 0xE6, 0x3F, 0x9C, 0xF7, 0xF6, 0x57, 0x26, 0x02, 0x59, 0x04, 0x74, 0xCC,
]);

// ----------------------------------------------------------------------------
// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blueprint_id() {
        let id = ViewId::random();
        let path = id.as_entity_path();
        assert!(path.is_child_of(&EntityPath::parse_forgiving("view/")));

        let id = ContainerId::random();
        let path = id.as_entity_path();
        assert!(path.is_child_of(&EntityPath::parse_forgiving("container/")));

        let roundtrip = ContainerId::from_entity_path(&id.as_entity_path());
        assert_eq!(roundtrip, id);

        let crossed = ContainerId::from_entity_path(&ViewId::random().as_entity_path());
        assert_eq!(crossed, ContainerId::invalid());
    }

    #[test]
    fn test_blueprint_id_string_serde() {
        #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        struct EventIds {
            #[serde(with = "blueprint_id_serde")]
            view_id: ViewId,
            #[serde(with = "blueprint_id_serde")]
            container_id: ContainerId,
        }

        let ids = EventIds {
            view_id: ViewId::random(),
            container_id: ContainerId::random(),
        };
        let value = serde_json::to_value(&ids).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "view_id": ids.view_id.uuid().to_string(),
                "container_id": ids.container_id.uuid().to_string(),
            }),
        );
        assert_eq!(
            serde_json::from_value::<EventIds>(value.clone()).unwrap(),
            ids
        );

        for field in ["view_id", "container_id"] {
            let mut invalid = value.clone();
            invalid[field] = serde_json::json!("not-a-uuid");
            assert!(serde_json::from_value::<EventIds>(invalid).is_err());
        }
    }

    #[test]
    fn test_blueprint_id_default_serde_is_unchanged() {
        let view_id = ViewId::random();
        let view_value = serde_json::to_value(view_id).unwrap();
        assert_eq!(
            view_value,
            serde_json::json!({ "id": view_id.uuid().to_string() })
        );
        assert_eq!(
            serde_json::from_value::<ViewId>(view_value).unwrap(),
            view_id
        );

        let container_id = ContainerId::random();
        let container_value = serde_json::to_value(container_id).unwrap();
        assert_eq!(
            container_value,
            serde_json::json!({ "id": container_id.uuid().to_string() })
        );
        assert_eq!(
            serde_json::from_value::<ContainerId>(container_value).unwrap(),
            container_id
        );
    }
}
