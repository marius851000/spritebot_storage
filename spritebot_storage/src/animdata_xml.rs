use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AnimDataXML {
    pub shadow_size: u8,
    pub anims: AnimsXML
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AnimsXML {
    pub anim: Vec<AnimXML>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AnimXML {
    //TODO: the server implementation seems to differ a bit to this.
    //TODO: also, handle the CopyOf properly
    pub name: String,
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rush_frame: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_frame: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_frame: Option<u32>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub durations: DurationsXML
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DurationsXML {
    pub duration: Vec<usize>
}