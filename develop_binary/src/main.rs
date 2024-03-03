use std::env::args;

use spritebot_storage::Sprite;
use vfs::PhysicalFS;

fn main() {
    let sprite_path = args().skip(1).next().unwrap();
    let mut vfs = PhysicalFS::new(&sprite_path);

    let mut sprite = Sprite::new(&mut vfs).unwrap();

    sprite.animations[0].images[7][0].offsets.center = (100, 100);

    let mut dest_vfs = PhysicalFS::new("./test");

    sprite.write_to_folder(&mut dest_vfs).unwrap();
}
