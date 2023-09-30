use bevy::{asset::Asset, prelude::*, reflect::*};
use std::marker::PhantomData;

#[derive(Resource)]
pub struct AssetHandle<T, H>
where
    H: TypeUuid + TypePath + Asset,
{
    pub handle: Handle<H>,
    asset_type: PhantomData<T>,
}

impl<T, H> AssetHandle<T, H>
where
    H: TypeUuid + TypePath + Asset,
{
    pub fn new(handle: Handle<H>) -> Self {
        Self {
            handle: handle,
            asset_type: PhantomData,
        }
    }
}
