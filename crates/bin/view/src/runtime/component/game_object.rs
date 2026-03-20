use super::{components::SceneComponent, sanitize_ascii_label, transform::SceneElementTransform};

fn default_game_object_name_for_id(id: GameObjectId) -> String {
    format!("GameObject {}", id.0)
}

fn default_game_object_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct GameObjectId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinGameObjectKind {
    MainCamera,
    Sun,
    LocalLights,
}

impl BuiltinGameObjectKind {
    pub fn display_name(self) -> &'static str {
        match self {
            BuiltinGameObjectKind::MainCamera => "Main Camera",
            BuiltinGameObjectKind::Sun => "Sun",
            BuiltinGameObjectKind::LocalLights => "Local Lights",
        }
    }

    pub fn matches_component(self, component: &SceneComponent) -> bool {
        matches!(
            (self, component),
            (BuiltinGameObjectKind::MainCamera, SceneComponent::Camera(_))
                | (BuiltinGameObjectKind::Sun, SceneComponent::Sun(_))
                | (BuiltinGameObjectKind::LocalLights, SceneComponent::LocalLights(_))
        )
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GameObject {
    pub id: GameObjectId,
    pub name: String,

    #[serde(default = "default_game_object_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub builtin: Option<BuiltinGameObjectKind>,

    #[serde(default)]
    pub parent: Option<GameObjectId>,

    #[serde(default)]
    pub transform: SceneElementTransform,

    #[serde(default)]
    pub components: Vec<SceneComponent>,
}

impl GameObject {
    pub fn new(
        id: GameObjectId,
        name: impl Into<String>,
        transform: SceneElementTransform,
    ) -> Self {
        let name = Self::sanitized_name(id, &name.into());
        Self {
            id,
            name,
            enabled: true,
            builtin: None,
            parent: None,
            transform,
            components: Vec::new(),
        }
    }

    pub fn sanitized_name(id: GameObjectId, name: &str) -> String {
        let sanitized = sanitize_ascii_label(name);
        if sanitized.is_empty() {
            default_game_object_name_for_id(id)
        } else {
            sanitized
        }
    }

    pub fn is_builtin(&self) -> bool {
        self.builtin.is_some()
    }
}