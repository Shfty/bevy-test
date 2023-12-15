use bevy::{
    prelude::{HandleUntyped, Shader},
    reflect::TypeUuid,
};

pub const SDF_2D_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7860297731842142478);
