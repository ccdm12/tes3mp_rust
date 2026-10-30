# TES3MP Bevy

A reimplementation of OpenMW / TES3MP built in Rust, using the [Bevy](https://bevy.org) game engine.

## Demo: 
[![Demo link](https://img.youtube.com/vi/jQ5HvB0AKjA/hqdefault.jpg)](https://www.youtube.com/watch?v=jQ5HvB0AKjA)


Mainly exists because TES3MP doesn't even compile with modern compilers, and doesn't even run on cachyos. Also, because this would make massive mods and new systems much easier to add.

## Current Features

* **ESM/ESP Loading -** Loads most Esm types, doesn't support saving yet.
* **World -** Nearby landscape/cell generation, almost fully complete cell generation, no interactions yet besides collisions.
* **Animation -** Uses [Bevy_Nif](https://github.com/christopherverch/bevy_nif) to load nif files and turn them into bevy assets. All player animations work, but it's a bit hardcoded so creatures probably don't work yet.
* **Web Support -** Can currently run on web using wasm and webgl2, if I happen to be hosting you can play it [here](http://107.136.161.231:8080/), but give it a while to load since the client.wasm is ~230mb.

## Controls
Enter - lock/unlock camera  
Mouse - look around  
Q - autorun / swap selected weapon animation  
Ctrl - crouch  
Capslock - toggle run  

## Building
**Note: This uses mold to compile faster, if you don't have it or don't want to use it, remove everything below the -------- in .cargo/config.toml**  

**Linux -** you should just be able to do cargo run --bin client, the default is set to x11, if you're on wayland change the "x11" feature to "wayland" in apps/client/Cargo.toml  

**Windows -** run cargo build --target x86_64-pc-windows-gnu, cargo might complain about some dependencies, just look up the errors and download what's missing.   

**Web -** You need to download bevy_nif and put it in the same directory as the tes3mp folder, and in tes3mp/Cargo.toml, replace bevy_nif = ... with bevy_nif = { path = "../bevy_nif/bevy_nif/"} because it has a build script that looks in ../../tes3mp/apps/client/assets to prebuild the asset paths it will use.   
First run cargo wasm in the tes3mp folder, then wasm-bindgen --out-name wasm_example --out-dir examples/wasm/target --target web target/wasm32-unknown-unknown/wasm-release/client.wasm && python3 -m http.server 8080 --directory examples/wasm/ and it will host on localhost:8080  
Note that you need to move the assets folder to tes3mp/examples/wasm/assets/

