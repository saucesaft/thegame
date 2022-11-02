use std::io::{BufReader, Cursor};

use wgpu::util::DeviceExt;

use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let base = reqwest::Url::parse(&format!(
        "{}/{}/",
        location.origin().unwrap(),
        option_env!("RES_PATH").unwrap_or("res"),
    )).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {

    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let txt = std::fs::read_to_string(path)?;

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {

    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let data = std::fs::read(path)?;

    Ok(data)
}


pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();

    if obj_materials?.is_empty() {
        let diffuse_texture = load_texture("no_texture.png", device, queue).await?;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(model::Material {
            name: "no_material".to_string(),

            diffuse_texture,

            bind_group,
        })
    };


    // for m in obj_materials? {
    //     let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
    //     let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //         layout,
    //         entries: &[
    //             wgpu::BindGroupEntry {
    //                 binding: 0,
    //                 resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 1,
    //                 resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
    //             },
    //         ],
    //         label: None,
    //     });

    //     materials.push(model::Material {
    //         name: m.name,
    //         diffuse_texture,
    //         bind_group,
    //     })
    // }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {

                // let mut t: [f32; 2] = [0.0, 0.0];
                // let mut n: [f32; 3] = [0.0, 0.0, 0.0];

                // if !m.mesh.texcoords.len() == 0 {
                //     println!("inside texcoords");
                //     t = [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]];
                //     println!("{}", i * 2);
                //     println!("{}", i * 2 + 1);
                //     println!("--------------");
                // };


                // if !m.mesh.normals.len() == 0 {
                //     n = [
                //         m.mesh.normals[i * 3],
                //         m.mesh.normals[i * 3 + 1],
                //         m.mesh.normals[i * 3 + 2],
                //     ];
                //     println!("inside normals");
                // };

                let mut normal: [f32; 3] = [0.0, 0.0, 0.0];
                if !m.mesh.normals.is_empty() {
                    // normal = [x, y, z]
                    normal = [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ];
                }

                let mut texcoord: [f32; 2] = [0.0, 0.0];
                if !m.mesh.texcoords.is_empty() {
                    // texcoord = [u, v];
                    texcoord = [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]];
                }

                model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: texcoord,
                    normal: normal,
                } })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes, materials })
}
