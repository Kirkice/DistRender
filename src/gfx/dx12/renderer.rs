use std::mem::ManuallyDrop;
use tracing::{trace, debug, info};
use winit::event_loop::EventLoop;
use crate::gfx::Dx12Context;
use crate::gfx::backend::GraphicsBackend;
use crate::core::{Config, SceneConfig};
use crate::core::error::{Result, DistRenderError, GraphicsError};
use crate::renderer::resources::vertex::{MyVertex, create_default_triangle, convert_geometry_vertex};
use crate::renderer::resources::resource::FrameResourcePool;
use crate::renderer::commands::sync::{FenceManager, FenceValue};
use crate::gfx::dx12::descriptor::Dx12DescriptorManager;
use crate::geometry::loaders::{MeshLoader, ObjLoader};
use crate::component::{Camera, DirectionalLight};
use crate::math::{Vector3, Matrix4};
use crate::gui::ipc::GuiStatePacket;
use std::path::Path;
use std::f32::consts::PI;
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_SWAP_CHAIN_FLAG, Common::*};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Threading::WaitForSingleObject;

const FRAME_COUNT: usize = 2;

/// Uniform Buffer Object - MVP 閻晠妯€閺佺増宓?
///
/// D3D12 鐟曚焦鐪扮敮鎼佸櫤缂傛挸鍟块崠?256 鐎涙濡€靛綊缍?
#[repr(C, align(256))]
#[derive(Clone, Copy, Debug)]
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    camera_pos: [f32; 4],
}

impl UniformBufferObject {
    fn new(model: &Matrix4, view: &Matrix4, projection: &Matrix4, light_dir:[f32;3], light_color:[f32;4], camera_pos:[f32;3]) -> Self {
        Self {
            model: *model.as_ref(),
            view: *view.as_ref(),
            projection: *projection.as_ref(),
            light_dir: [light_dir[0],light_dir[1],light_dir[2],0.0],
            light_color,
            camera_pos: [camera_pos[0],camera_pos[1],camera_pos[2],0.0],
        }
    }
}

pub struct Renderer {
    gfx: Dx12Context,
    root_signature: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    #[allow(dead_code)]  // 娣囨繄鏆€娓氭稑鐨㈤弶銉ゅ▏閻?
    vertex_buffer: ID3D12Resource,
    vertex_buffer_view: D3D12_VERTEX_BUFFER_VIEW,
    vertex_count: u32,
    #[allow(dead_code)]  // 娣囨繄鏆€娓氭稑鐨㈤弶銉ゅ▏閻?
    index_buffer: ID3D12Resource,
    index_buffer_view: D3D12_INDEX_BUFFER_VIEW,
    index_count: u32,
    viewport: D3D12_VIEWPORT,
    scissor_rect: RECT,
    command_allocators: [ID3D12CommandAllocator; FRAME_COUNT],
    command_list: ID3D12GraphicsCommandList,

    // 濞ｅ崬瀹?濡剝婢樼紓鎾冲暱
    depth_stencil_heap: ID3D12DescriptorHeap,
    depth_stencil_buffer: ID3D12Resource,

    // 娴ｈ法鏁ら弬鎵畱鐢嗙カ濠ф劗顓搁悶鍡欓兇缂佺噦绱欓弴澶稿敩fence_values閿?
    frame_resource_pool: FrameResourcePool,
    // 娴ｈ法鏁ら弬鎵畱Fence缁狅紕鎮婇崳?
    fence_manager: FenceManager,
    // 閹诲繗鍫粭锔绢吀閻炲棗娅?
    descriptor_manager: Dx12DescriptorManager,
    // 鐢悂鍣虹紓鎾冲暱閸栫尨绱橫VP 閻晠妯€閿?
    constant_buffer: ID3D12Resource,
    constant_buffer_data: *mut u8,
    // 閸︾儤娅欓柊宥囩枂
    scene: SceneConfig,
    // 閻╁憡婧€缂佸嫪娆?
    camera: Camera,
    // 閺傜懓鎮滈崗澶岀矋娴?
    directional_light: DirectionalLight,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &SceneConfig) -> Result<Self> {
        let gfx = Dx12Context::new(event_loop, config);

        unsafe {
            // 1. Root Signature閿涘牆瀵橀崥顐㈢埗闁插繒绱﹂崘鎻掑隘閹诲繗鍫粭锔肩礆
            let root_parameters = [
                D3D12_ROOT_PARAMETER {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
                    Anonymous: D3D12_ROOT_PARAMETER_0 {
                        Descriptor: D3D12_ROOT_DESCRIPTOR {
                            ShaderRegister: 0,  // b0
                            RegisterSpace: 0,
                        },
                    },
                    ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
                },
            ];

            let root_desc = D3D12_ROOT_SIGNATURE_DESC {
                NumParameters: root_parameters.len() as u32,
                pParameters: root_parameters.as_ptr(),
                NumStaticSamplers: 0,
                pStaticSamplers: std::ptr::null(),
                Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
            };

            let mut signature = None;
            D3D12SerializeRootSignature(&root_desc, D3D_ROOT_SIGNATURE_VERSION_1, &mut signature, None)
                .map_err(|e| DistRenderError::Graphics(
                    GraphicsError::ResourceCreation(format!("Failed to serialize root signature: {:?}", e))
                ))?;
            let signature = signature.unwrap();

            let root_signature: ID3D12RootSignature = gfx.device.CreateRootSignature(
                0,
                std::slice::from_raw_parts(signature.GetBufferPointer() as _, signature.GetBufferSize())
            ).map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create root signature: {:?}", e))
            ))?;

            // 2. Shaders閿涘牆鍨庨崚顐ヮ嚢閸欐牕鑻熺紓鏍槯 vertex.hlsl / fragment.hlsl閿?
            use std::fs;
            use std::path::PathBuf;

            // Windows 娑撳浼愭担婊呮窗瑜版洖褰查懗鎴掔瑝閺勵垶銆嶉惄顔界壌閻╊喖缍嶉敍灞肩瑝閼崇晫娲块幒銉ょ贩鐠ф牜娴夌€电鐭惧鍕┾偓?
            // 閻劎绱拠鎴炴埂妞ゅ湱娲伴弽鍦窗瑜版洩绱機ARGO_MANIFEST_DIR閿涘娼电€规矮缍?shader 閺傚洣娆㈤妴?
            let shader_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/gfx/dx12/shaders");

            let vs_path = shader_dir.join("vertex.hlsl");
            let ps_path = shader_dir.join("fragment.hlsl");

            let vs_hlsl = fs::read_to_string(&vs_path)
                .unwrap_or_else(|e| panic!("Failed to read vertex.hlsl at {}: {}", vs_path.display(), e));
            let ps_hlsl = fs::read_to_string(&ps_path)
                .unwrap_or_else(|e| panic!("Failed to read fragment.hlsl at {}: {}", ps_path.display(), e));

            let mut vs_blob = None;
            let mut ps_blob = None;
            let mut error_blob = None;

            let result = D3DCompile(
                vs_hlsl.as_ptr() as _,
                vs_hlsl.len(),
                None,
                None,
                None,
                windows::core::s!("VSMain"),
                windows::core::s!("vs_5_0"),
                0,
                0,
                &mut vs_blob,
                Some(&mut error_blob),
            );
            if let Err(e) = result {
                if let Some(error) = error_blob {
                    let message = std::str::from_utf8(std::slice::from_raw_parts(
                        error.GetBufferPointer() as *const u8,
                        error.GetBufferSize(),
                    )).unwrap();
                    panic!("VS Compile Error: {}", message);
                }
                panic!("VS Compile Failed: {:?}", e);
            }

            let result = D3DCompile(
                ps_hlsl.as_ptr() as _,
                ps_hlsl.len(),
                None,
                None,
                None,
                windows::core::s!("PSMain"),
                windows::core::s!("ps_5_0"),
                0,
                0,
                &mut ps_blob,
                Some(&mut error_blob),
            );
            if let Err(e) = result {
                 if let Some(error) = error_blob {
                    let message = std::str::from_utf8(std::slice::from_raw_parts(
                        error.GetBufferPointer() as *const u8,
                        error.GetBufferSize(),
                    )).unwrap();
                    panic!("PS Compile Error: {}", message);
                }
                panic!("PS Compile Failed: {:?}", e);
            }
            let vs_blob = vs_blob.unwrap();
            let ps_blob = ps_blob.unwrap();

            // 3. Input Layout (POSITION/NORMAL/COLOR)
            let input_element_descs = [
                D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: windows::core::s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: windows::core::s!("NORMAL"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 12,
                    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: windows::core::s!("COLOR"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 24,
                    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ];

            // 4. PSO
            let mut pso_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC::default();
            pso_desc.pRootSignature = std::mem::transmute_copy(&root_signature); // ManuallyDrop wrapper might be needed here if strict, but let's try direct assign for Option wrapper
            // Wait, ID3D12RootSignature is a pointer, but the struct expects "Option<ID3D12RootSignature>".
            // Actually, in windows-rs, COM interfaces in structs are often wrapped in ManuallyDrop if they are in unions or directly Option<T>.
            // Let's check D3D12_GRAPHICS_PIPELINE_STATE_DESC definition.
            
            pso_desc.pRootSignature = ManuallyDrop::new(Some(root_signature.clone()));
            pso_desc.VS = D3D12_SHADER_BYTECODE {
                pShaderBytecode: vs_blob.GetBufferPointer(),
                BytecodeLength: vs_blob.GetBufferSize(),
            };
            pso_desc.PS = D3D12_SHADER_BYTECODE {
                pShaderBytecode: ps_blob.GetBufferPointer(),
                BytecodeLength: ps_blob.GetBufferSize(),
            };
            pso_desc.BlendState = D3D12_BLEND_DESC {
                AlphaToCoverageEnable: false.into(),
                IndependentBlendEnable: false.into(),
                RenderTarget: [
                    D3D12_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: false.into(),
                        LogicOpEnable: false.into(),
                        RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
                        ..Default::default()
                    },
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                    D3D12_RENDER_TARGET_BLEND_DESC::default(),
                ],
            };
            pso_desc.RasterizerState = D3D12_RASTERIZER_DESC {
                FillMode: D3D12_FILL_MODE_SOLID,
                CullMode: D3D12_CULL_MODE_BACK,  // 閼冲矂娼伴崜鏃堟珟
                ..Default::default()
            };
            // 閸氼垳鏁ゅǎ鍗炲濞村鐦?
            pso_desc.DepthStencilState = D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: true.into(),
                DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ALL,
                DepthFunc: D3D12_COMPARISON_FUNC_LESS,  // 濞ｅ崬瀹抽崐鐓庣毈閻ㄥ嫰鈧俺绻冮敍鍫熸纯鏉╂垹娈戦悧鈺€缍嬮敍?
                StencilEnable: false.into(),
                StencilReadMask: 0xFF,
                StencilWriteMask: 0xFF,
                FrontFace: D3D12_DEPTH_STENCILOP_DESC::default(),
                BackFace: D3D12_DEPTH_STENCILOP_DESC::default(),
            };
            pso_desc.SampleMask = 0xFFFFFFFF;
            pso_desc.DSVFormat = DXGI_FORMAT_D32_FLOAT;  // 32娴ｅ秵璇為悙瑙勭箒鎼达附鐗稿?
            pso_desc.InputLayout = D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: input_element_descs.as_ptr(),
                NumElements: input_element_descs.len() as u32,
            };
            pso_desc.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
            pso_desc.NumRenderTargets = 1;
            pso_desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
            pso_desc.SampleDesc.Count = 1;

            let pso: ID3D12PipelineState = gfx.device.CreateGraphicsPipelineState(&pso_desc).expect("Failed to create PSO");

            // 5. MyVertex Buffer - 閸旂姾娴?OBJ 濡€崇€烽弬鍥︽
            let obj_path = Path::new("assets/models/sphere.obj");
            let (vertices, indices) = if obj_path.exists() {
                info!("Loading mesh from: {}", obj_path.display());
                match ObjLoader::load_from_file(obj_path) {
                    Ok(mesh_data) => {
                        info!(
                            "Mesh loaded successfully: {} vertices, {} indices",
                            mesh_data.vertex_count(),
                            mesh_data.index_count()
                        );
                        // 鏉烆剚宕?GeometryVertex 娑?MyVertex
                        let verts = mesh_data
                            .vertices
                            .iter()
                            .map(|v| convert_geometry_vertex(v))
                            .collect::<Vec<_>>();
                        let inds = mesh_data.indices.clone();
                        (verts, inds)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load OBJ file: {}, using default triangle", e);
                        (create_default_triangle().to_vec(), vec![0, 1, 2])
                    }
                }
            } else {
                tracing::warn!("OBJ file not found: {}, using default triangle", obj_path.display());
                (create_default_triangle().to_vec(), vec![0, 1, 2])
            };
            let vertex_data_size = (std::mem::size_of::<MyVertex>() * vertices.len()) as u64;

            let heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_UPLOAD,
                ..Default::default()
            };
            let resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: vertex_data_size,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                ..Default::default()
            };

            let mut vertex_buffer: Option<ID3D12Resource> = None;
            gfx.device.CreateCommittedResource(
                &heap_props,
                D3D12_HEAP_FLAG_NONE,
                &resource_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut vertex_buffer,
            ).expect("Failed to create VB");
            let vertex_buffer = vertex_buffer.unwrap();

            // Copy data
            let mut data = std::ptr::null_mut();
            vertex_buffer.Map(0, None, Some(&mut data)).unwrap();
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), data as *mut MyVertex, vertices.len());
            vertex_buffer.Unmap(0, None);

            let vertex_buffer_view = D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: vertex_buffer.GetGPUVirtualAddress(),
                SizeInBytes: vertex_data_size as u32,
                StrideInBytes: std::mem::size_of::<MyVertex>() as u32,
            };

            let vertex_count = vertices.len() as u32;

            // 5.5. 閸掓稑缂撶槐銏犵穿缂傛挸鍟块崠鐚寸礄Index Buffer閿?
            let index_data_size = (std::mem::size_of::<u32>() * indices.len()) as u64;
            let index_count = indices.len() as u32;

            let ib_resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: index_data_size,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                ..Default::default()
            };

            let mut index_buffer: Option<ID3D12Resource> = None;
            gfx.device.CreateCommittedResource(
                &heap_props,
                D3D12_HEAP_FLAG_NONE,
                &ib_resource_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut index_buffer,
            ).expect("Failed to create IB");
            let index_buffer = index_buffer.unwrap();

            // Copy index data
            let mut ib_data = std::ptr::null_mut();
            index_buffer.Map(0, None, Some(&mut ib_data)).unwrap();
            std::ptr::copy_nonoverlapping(indices.as_ptr(), ib_data as *mut u32, indices.len());
            index_buffer.Unmap(0, None);

            let index_buffer_view = D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: index_buffer.GetGPUVirtualAddress(),
                SizeInBytes: index_data_size as u32,
                Format: DXGI_FORMAT_R32_UINT,
            };

            info!("Index buffer created: {} indices", index_count);

            // 5.6. 閸掓稑缂撶敮鎼佸櫤缂傛挸鍟块崠鐚寸礄Constant Buffer for MVP matrices閿?
            let constant_buffer_size = std::mem::size_of::<UniformBufferObject>() as u64;

            let cb_heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_UPLOAD,
                ..Default::default()
            };
            let cb_resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: constant_buffer_size,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                ..Default::default()
            };

            let mut constant_buffer: Option<ID3D12Resource> = None;
            gfx.device.CreateCommittedResource(
                &cb_heap_props,
                D3D12_HEAP_FLAG_NONE,
                &cb_resource_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut constant_buffer,
            ).expect("Failed to create constant buffer");
            let constant_buffer = constant_buffer.unwrap();

            // Map 鐢悂鍣虹紓鎾冲暱閸栬桨浜掗懢宄板絿 CPU 閹稿洭鎷?
            let mut constant_buffer_data = std::ptr::null_mut();
            constant_buffer.Map(0, None, Some(&mut constant_buffer_data)).unwrap();

            info!("Constant buffer created and mapped (size: {} bytes)", constant_buffer_size);

            // 6. Viewport/Scissor
             let viewport = D3D12_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: gfx.width as f32,
                Height: gfx.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };

            let scissor_rect = RECT {
                left: 0,
                top: 0,
                right: gfx.width as i32,
                bottom: gfx.height as i32,
            };

            // 7. 閸掓稑缂撻崨鎴掓姢鐎电钖勯敍鍫濆蓟缂傛挸鍟块敍?
            #[cfg(debug_assertions)]
            debug!(frame_count = FRAME_COUNT, "Creating command allocators for frame buffering");

            let command_allocators: [ID3D12CommandAllocator; FRAME_COUNT] = [
                gfx.device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
                    .expect("Failed to create CommandAllocator 0"),
                gfx.device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
                    .expect("Failed to create CommandAllocator 1"),
            ];

            let command_list: ID3D12GraphicsCommandList =
                gfx.device.CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT,
                    &command_allocators[0],
                    Some(&pso)
                ).expect("Failed to create CommandList");

            // 閸掓繂顫愰崚娑樼紦閺冭泛鎳℃禒銈呭灙鐞涖劍妲搁幍鎾崇磻閻樿埖鈧緤绱濋棁鈧憰浣稿帥閸忔娊妫?
            command_list.Close().expect("Failed to close initial CommandList");

            // 閸掓繂顫愰崠鏍ф姎鐠у嫭绨Ч鐙呯礄閸欏瞼绱﹂崘璇х礉娑撳锭RAME_COUNT閸栧綊鍘ら敍?
            let frame_resource_pool = FrameResourcePool::double_buffering();

            // 閸掓繂顫愰崠鏈廵nce缁狅紕鎮婇崳?
            let fence_manager = FenceManager::new();

            // 閸掓繂顫愰崠鏍ㄥ伎鏉╂壆顑佺粻锛勬倞閸?
            let mut descriptor_manager = Dx12DescriptorManager::new();

            // 閸掓繂顫愰崠?RTV 閸棴绱欐禍銈嗗床闁惧墽绱﹂崘鍙夋殶闁插骏绱?
            descriptor_manager.init_rtv_heap(&gfx.device, FRAME_COUNT as u32)?;

            // 閸掓繂顫愰崠?DSV 閸棴绱欓懛鍐茬毌1娑擃亝绻佹惔锔剧处閸愯绱?
            descriptor_manager.init_dsv_heap(&gfx.device, 1)?;

            // 閸掓繂顫愰崠?SRV/CBV/UAV 閸棴绱欐０鍕瀻闁?28娑擃亝寮挎潻鎵儊閿涘苯寮懓?DistEngine閿?
            descriptor_manager.init_srv_cbv_uav_heap(&gfx.device, 128)?;

            // 閸掓稑缂撳ǎ鍗炲濡剝婢橀崼鍡礄閸楁洜瀚惃鍕垻閻劋绨珼SV閿?
            let dsv_heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
                NumDescriptors: 1,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };
            let depth_stencil_heap: ID3D12DescriptorHeap = gfx.device
                .CreateDescriptorHeap(&dsv_heap_desc)
                .expect("Failed to create DSV heap");

            // 閸掓稑缂撳ǎ鍗炲濡剝婢樼紓鎾冲暱鐠у嫭绨?
            let depth_heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                ..Default::default()
            };
            let depth_resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Width: gfx.width as u64,
                Height: gfx.height,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: DXGI_FORMAT_D32_FLOAT,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                Flags: D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,
                ..Default::default()
            };

            let clear_value = D3D12_CLEAR_VALUE {
                Format: DXGI_FORMAT_D32_FLOAT,
                Anonymous: D3D12_CLEAR_VALUE_0 {
                    DepthStencil: D3D12_DEPTH_STENCIL_VALUE {
                        Depth: 1.0,
                        Stencil: 0,
                    },
                },
            };

            let mut depth_stencil_buffer: Option<ID3D12Resource> = None;
            gfx.device.CreateCommittedResource(
                &depth_heap_props,
                D3D12_HEAP_FLAG_NONE,
                &depth_resource_desc,
                D3D12_RESOURCE_STATE_DEPTH_WRITE,
                Some(&clear_value),
                &mut depth_stencil_buffer,
            ).expect("Failed to create depth stencil buffer");
            let depth_stencil_buffer = depth_stencil_buffer.unwrap();

            // 閸掓稑缂撳ǎ鍗炲濡剝婢樼憴鍡楁禈
            gfx.device.CreateDepthStencilView(
                &depth_stencil_buffer,
                None,
                depth_stencil_heap.GetCPUDescriptorHandleForHeapStart(),
            );

            #[cfg(debug_assertions)]
            {
                info!("DX12 Renderer initialized successfully with double buffering");
                debug!("Descriptor heaps initialized: RTV={}, DSV={}, SRV/CBV/UAV={}",
                    FRAME_COUNT, 1, 128);
                info!("Depth stencil buffer created: {}x{}", gfx.width, gfx.height);
            }

            // 閸掓稑缂撻惄鍛婃簚缂佸嫪娆㈤敍鍫滅矤閸︾儤娅欓柊宥囩枂閸掓繂顫愰崠鏍电礆
            let mut camera = Camera::main_camera();
            camera.set_position(Vector3::new(
                scene.camera.transform.position[0],
                scene.camera.transform.position[1],
                scene.camera.transform.position[2],
            ));
            let aspect_ratio = viewport.Width / viewport.Height;
            camera.set_lens(
                scene.camera.fov * PI / 180.0,
                aspect_ratio,
                scene.camera.near_clip,
                scene.camera.far_clip,
            );

            // 婵″倹鐏夐張澶嬫鏉烆剨绱濇担璺ㄦ暏 look_at 鐠佸墽鐤嗛惄鍛婃簚閺堟繂鎮?
            let pitch = scene.camera.transform.rotation[0] * PI / 180.0;
            let yaw = scene.camera.transform.rotation[1] * PI / 180.0;
            let forward = Vector3::new(
                yaw.sin() * pitch.cos(),
                -pitch.sin(),
                -yaw.cos() * pitch.cos(),
            );
            let target = camera.position() + forward;
            camera.look_at(camera.position(), target, Vector3::new(0.0, 1.0, 0.0));

            info!("Camera component initialized at position {:?}", camera.position());

            // 閸掓繂顫愰崠鏍ㄦ煙閸氭垵鍘滅紒鍕
            let directional_light = scene.light.to_directional_light("MainLight");
            info!(
                "DirectionalLight component initialized: color={:?}, intensity={}, direction={:?}",
                directional_light.color.to_array(),
                directional_light.intensity,
                directional_light.direction
            );

            Ok(Self {
                gfx,
                root_signature,
                pso,
                vertex_buffer,
                vertex_buffer_view,
                vertex_count,
                index_buffer,
                index_buffer_view,
                index_count,
                viewport,
                scissor_rect,
                command_allocators,
                command_list,
                depth_stencil_heap,
                depth_stencil_buffer,
                frame_resource_pool,
                fence_manager,
                descriptor_manager,
                constant_buffer,
                constant_buffer_data: constant_buffer_data as *mut u8,
                scene: scene.clone(),
                camera,
                directional_light,
            })
        }
    }

    /// 缁涘绶烥PU鐎瑰本鍨氶幍鈧張澶婁紣娴ｆ粣绱欑猾璁虫妧DistEngine閻ㄥ嚔lushCommandQueue閿?
    ///
    /// 鏉╂瑦妲告稉鈧稉顏堟▎婵夌偞鎼锋担婊愮礉娴兼氨鐡戝鍛閺堝褰佹禍銈囨畱GPU閸涙垝鎶ょ€瑰本鍨氶妴?
    /// 闁艾鐖堕悽銊ょ艾濞撳懐鎮婄挧鍕爱閹存牕鎮撳銉у仯閵?
    pub fn flush(&mut self) -> Result<()> {
        unsafe {
            #[cfg(debug_assertions)]
            debug!("Flushing DX12 command queue...");

            // Signal娑撯偓娑擃亝鏌婇惃鍒nce閸?
            let flush_fence = self.fence_manager.next_value();
            self.gfx.command_queue.Signal(&self.gfx.fence, flush_fence.value())
                .expect("Failed to signal fence");

            // 缁涘绶熺拠顧琫nce閸婄厧鐣幋?
            if self.gfx.fence.GetCompletedValue() < flush_fence.value() {
                self.gfx.fence.SetEventOnCompletion(flush_fence.value(), self.gfx.fence_event)
                    .expect("Failed to set fence event");
                WaitForSingleObject(self.gfx.fence_event, windows::Win32::System::Threading::INFINITE);
            }

            // 閺囧瓨鏌奻ence缁狅紕鎮婇崳?
            self.fence_manager.update_completed_value(flush_fence);

            // 閺囧瓨鏌婇幍鈧張澶婃姎鐠у嫭绨稉鍝勫讲閻?
            self.frame_resource_pool.update_availability(flush_fence.value());

            #[cfg(debug_assertions)]
            debug!("DX12 command queue flushed");

            Ok(())
        }
    }

    pub fn resize(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            debug!("Resizing swapchain...");

            // 缁涘绶?GPU 鐎瑰本鍨氶幍鈧張澶婁紣娴?
            let fence_value = self.gfx.fence_value;
            self.gfx.command_queue.Signal(&self.gfx.fence, fence_value)
                .expect("Failed to signal fence for resize");
            self.gfx.fence_value += 1;

            if self.gfx.fence.GetCompletedValue() < fence_value {
                self.gfx.fence.SetEventOnCompletion(fence_value, self.gfx.fence_event)
                    .expect("Failed to set fence event for resize");
                WaitForSingleObject(self.gfx.fence_event, windows::Win32::System::Threading::INFINITE);
            }

            #[cfg(debug_assertions)]
            debug!("GPU idle, resizing swap chain buffers...");

            // 閼惧嘲褰囬弬鎵畱缁愭褰涙径褍鐨?
            let size = self.gfx.window.inner_size();
            self.gfx.width = size.width;
            self.gfx.height = size.height;

            // 鐠嬪啯鏆ｆ禍銈嗗床闁炬儳銇囩亸蹇ョ礄娴兼俺鍤滈崝銊╁櫞閺€鐐＋閻ㄥ嫮绱﹂崘鎻掑隘閿?
            self.gfx.swap_chain.ResizeBuffers(
                FRAME_COUNT as u32,
                size.width,
                size.height,
                DXGI_FORMAT_R8G8B8A8_UNORM,
                DXGI_SWAP_CHAIN_FLAG(0),
            ).expect("Failed to resize swap chain buffers");

            // 闁插秵鏌婇崚娑樼紦 RTV
            let rtv_handle = self.gfx.rtv_heap.GetCPUDescriptorHandleForHeapStart();
            for i in 0..FRAME_COUNT {
                let surface: ID3D12Resource = self.gfx.swap_chain.GetBuffer(i as u32)
                    .expect("Failed to get swap chain buffer");
                let handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + (i * self.gfx.rtv_descriptor_size),
                };
                self.gfx.device.CreateRenderTargetView(&surface, None, handle);
            }

            // 闁插秵鏌婇崚娑樼紦濞ｅ崬瀹冲Ο鈩冩緲缂傛挸鍟?
            let depth_heap_props = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                ..Default::default()
            };
            let depth_resource_desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Width: size.width as u64,
                Height: size.height,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: DXGI_FORMAT_D32_FLOAT,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                Flags: D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,
                ..Default::default()
            };

            let clear_value = D3D12_CLEAR_VALUE {
                Format: DXGI_FORMAT_D32_FLOAT,
                Anonymous: D3D12_CLEAR_VALUE_0 {
                    DepthStencil: D3D12_DEPTH_STENCIL_VALUE {
                        Depth: 1.0,
                        Stencil: 0,
                    },
                },
            };

            let mut new_depth_buffer: Option<ID3D12Resource> = None;
            self.gfx.device.CreateCommittedResource(
                &depth_heap_props,
                D3D12_HEAP_FLAG_NONE,
                &depth_resource_desc,
                D3D12_RESOURCE_STATE_DEPTH_WRITE,
                Some(&clear_value),
                &mut new_depth_buffer,
            ).expect("Failed to create depth stencil buffer during resize");
            self.depth_stencil_buffer = new_depth_buffer.unwrap();

            // 闁插秵鏌婇崚娑樼紦濞ｅ崬瀹冲Ο鈩冩緲鐟欏棗娴?
            self.gfx.device.CreateDepthStencilView(
                &self.depth_stencil_buffer,
                None,
                self.depth_stencil_heap.GetCPUDescriptorHandleForHeapStart(),
            );

            // 閺囧瓨鏌?viewport 閸?scissor rect
            self.viewport.Width = size.width as f32;
            self.viewport.Height = size.height as f32;
            self.scissor_rect.right = size.width as i32;
            self.scissor_rect.bottom = size.height as i32;

            // 闁插秶鐤?frame index
            self.gfx.frame_index = self.gfx.swap_chain.GetCurrentBackBufferIndex() as usize;

            // 濞撳懘娅?fence 閸婄》绱欓崶鐘辫礋閹存垳婊戠粵澶婄窡娴滃棙澧嶉張澶婃姎鐎瑰本鍨氶敍?
            // 闁插秶鐤嗙敮褑绁┃鎰潨
            self.frame_resource_pool = FrameResourcePool::double_buffering();
            self.fence_manager.reset();

            #[cfg(debug_assertions)]
            debug!(width = size.width, height = size.height, "Resize completed");
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        unsafe {
            let frame_index = self.gfx.frame_index;

            #[cfg(debug_assertions)]
            {
                let completed_value = self.gfx.fence.GetCompletedValue();
                trace!(frame_index, fence_value = self.gfx.fence_value, completed = completed_value, "Frame state");
            }

            // 娴ｈ法鏁ら弬鎵畱鐢嗙カ濠ф劗顓搁悶鍡窗濡偓閺屻儱缍嬮崜宥呮姎鐠у嫭绨弰顖氭儊閸欘垳鏁?
            let current_frame_resource = self.frame_resource_pool.get(frame_index)
                .ok_or_else(|| DistRenderError::Runtime("Invalid frame index".to_string()))?;
            if !current_frame_resource.available {
                let fence_value = current_frame_resource.fence_value;

                #[cfg(debug_assertions)]
                debug!(frame_index, fence_value, "Waiting for GPU (frame resource in use)");

                // 缁涘绶熺拠銉ユ姎鐠у嫭绨€瑰本鍨?
                if self.gfx.fence.GetCompletedValue() < fence_value {
                    self.gfx.fence.SetEventOnCompletion(fence_value, self.gfx.fence_event)
                        .expect("Failed to set fence event");
                    WaitForSingleObject(self.gfx.fence_event, windows::Win32::System::Threading::INFINITE);

                    #[cfg(debug_assertions)]
                    debug!(frame_index, "GPU wait completed");
                }

                // 閺囧瓨鏌婂鎻掔暚閹存劗娈慺ence閸?
                self.fence_manager.update_completed_value(FenceValue::new(self.gfx.fence.GetCompletedValue()));
                self.frame_resource_pool.update_availability(self.gfx.fence.GetCompletedValue());
            }

            // 闁插秶鐤嗚ぐ鎾冲鐢呮畱閸涙垝鎶ら崚鍡涘帳閸ｃ劌鎷伴崨鎴掓姢閸掓銆?
            let allocator = &self.command_allocators[frame_index];
            allocator.Reset().expect("Failed to reset CommandAllocator");
            self.command_list.Reset(allocator, Some(&self.pso))
                .expect("Failed to reset CommandList");

            // 閺囧瓨鏌婇惄鍛婃簚閻ㄥ嫬顔旀妯荤槷閿涘牆顩ч弸婊呯崶閸欙絽銇囩亸蹇旀暭閸欐﹫绱?
            let aspect_ratio = self.viewport.Width / self.viewport.Height;
            self.camera.set_aspect(aspect_ratio);

            // 鐠侊紕鐣?MVP 閻晠妯€閿涘牅濞囬悽?Camera 缂佸嫪娆㈤敍?
            let model = self.scene.model.transform.to_matrix();
            let view = self.camera.view_matrix();
            let mut projection = self.camera.proj_matrix();
            projection[(1, 1)] *= -1.0;
            
            // 娴ｈ法鏁?DirectionalLight 缂佸嫪娆㈤懢宄板絿閻忣垰鍘滈崣鍌涙殶
            let light_direction = self.directional_light.direction;
            let light_color_intensity = self.directional_light.color.with_intensity(self.directional_light.intensity);

            let camera_pos = self.camera.position();
            let ubo = UniformBufferObject::new(
                &model,
                &view,
                &projection,
                [light_direction.x, light_direction.y, light_direction.z],
                [light_color_intensity[0], light_color_intensity[1], light_color_intensity[2], self.directional_light.intensity],
                [camera_pos.x, camera_pos.y, camera_pos.z],
            );

            // 閺囧瓨鏌婄敮鎼佸櫤缂傛挸鍟块崠鐑樻殶閹?
            std::ptr::copy_nonoverlapping(
                &ubo as *const UniformBufferObject as *const u8,
                self.constant_buffer_data,
                std::mem::size_of::<UniformBufferObject>()
            );

            // Get render target resource
            let render_target: ID3D12Resource = self.gfx.swap_chain.GetBuffer(self.gfx.frame_index as u32)
                .map_err(|e| DistRenderError::Graphics(GraphicsError::ResourceCreation(format!("Failed to get swap chain buffer: {:?}", e))))?;

            // Transition Barrier Present -> RenderTarget
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(Some(render_target.clone())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_PRESENT,
                        StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET,
                    }),
                },
            };
            self.command_list.ResourceBarrier(&[barrier]);

            // 鐠佸墽鐤嗗〒鍙夌厠閻╊喗鐖ｉ崪灞剧箒鎼达附膩閺?
            let rtv_handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.gfx.rtv_heap.GetCPUDescriptorHandleForHeapStart().ptr + (self.gfx.frame_index * self.gfx.rtv_descriptor_size),
            };
            let dsv_handle = self.depth_stencil_heap.GetCPUDescriptorHandleForHeapStart();

            self.command_list.OMSetRenderTargets(1, Some(&rtv_handle), false, Some(&dsv_handle));

            // 濞撳懐鈹栧〒鍙夌厠閻╊喗鐖ｉ崪灞剧箒鎼达妇绱﹂崘?
            self.command_list.ClearRenderTargetView(rtv_handle, &self.scene.clear_color, None);
            self.command_list.ClearDepthStencilView(
                dsv_handle,
                D3D12_CLEAR_FLAG_DEPTH,
                1.0,  // 濞ｅ崬瀹冲〒鍛敄娑?.0閿涘牊娓舵潻婊愮礆
                0,
                None,
            );

            // Draw
            self.command_list.SetGraphicsRootSignature(&self.root_signature);
            self.command_list.SetPipelineState(&self.pso);
            self.command_list.RSSetViewports(&[self.viewport]);
            self.command_list.RSSetScissorRects(&[self.scissor_rect]);

            // 鐠佸墽鐤嗙敮鎼佸櫤缂傛挸鍟块崠鐚寸礄Root Parameter 0閿?
            self.command_list.SetGraphicsRootConstantBufferView(
                0,  // Root parameter index
                self.constant_buffer.GetGPUVirtualAddress()
            );

            self.command_list.OMSetRenderTargets(1, Some(&rtv_handle), false, None);
            self.command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.command_list.IASetVertexBuffers(0, Some(&[self.vertex_buffer_view]));
            self.command_list.IASetIndexBuffer(Some(&self.index_buffer_view));
            self.command_list.DrawIndexedInstanced(self.index_count, 1, 0, 0, 0);

            // Transition Barrier RenderTarget -> Present
            let barrier_back = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(Some(render_target.clone())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_RENDER_TARGET,
                        StateAfter: D3D12_RESOURCE_STATE_PRESENT,
                    }),
                },
            };
            self.command_list.ResourceBarrier(&[barrier_back]);

            // Explicitly drop the render target to release reference before potential resize
            drop(render_target);

            self.command_list.Close()
                .expect("Failed to close command list");

            #[cfg(debug_assertions)]
            trace!(frame_index, "Executing command list");

            // Execute
            let command_lists = [Some(self.command_list.clone().into())];
            self.gfx.command_queue.ExecuteCommandLists(&command_lists);

            // Present
            self.gfx.swap_chain.Present(1, DXGI_PRESENT(0)).ok()
                .expect("Failed to present");

            #[cfg(debug_assertions)]
            trace!(frame_index, "Presented");

            // 娴ｈ法鏁ら弬鎵畱 Fence 缁狅紕鎮婇崳銊﹀絹娴溿倓淇婇崣?
            let fence_value = self.fence_manager.next_value();
            self.gfx.command_queue.Signal(&self.gfx.fence, fence_value.value())
                .expect("Failed to signal fence");

            #[cfg(debug_assertions)]
            trace!(frame_index, fence_value = fence_value.value(), "Fence signaled");

            // 閺嶅洩顔囪ぐ鎾冲鐢嗙カ濠ф劒璐熸担璺ㄦ暏娑?
            if let Some(frame_resource) = self.frame_resource_pool.get_mut(frame_index) {
                frame_resource.mark_in_use(fence_value.value());
            }

            // 娣囨繃瀵旀稉?gfx.fence_value 閻ㄥ嫬鎮撳銉礄娑撹桨绨￠崗鐓庮啇閹嶇礆
            self.gfx.fence_value = fence_value.value() + 1;

            // Update frame index
            self.gfx.frame_index = self.gfx.swap_chain.GetCurrentBackBufferIndex() as usize;

            #[cfg(debug_assertions)]
            trace!(frame_index, next_frame = self.gfx.frame_index, "Frame completed");

            Ok(())
        }
    }

    /// Update camera based on input system state
    ///
    /// Called every frame before draw() to apply user input to camera
    pub fn update(&mut self, input_system: &mut crate::core::input::InputSystem, delta_time: f32) {
        input_system.update_camera(&mut self.camera, delta_time);
    }

    pub fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        self.scene.clear_color = packet.clear_color;
        self.scene.model.transform.position = packet.model_position;
        self.scene.model.transform.rotation = packet.model_rotation;
        self.scene.model.transform.scale = packet.model_scale;

        self.directional_light.intensity = packet.light_intensity;
        self.directional_light.direction = Vector3::new(
            packet.light_direction[0],
            packet.light_direction[1],
            packet.light_direction[2],
        )
        .normalize();

        if (self.camera.fov_x() - packet.camera_fov * PI / 180.0).abs() > 0.01 {
            self.camera.set_lens(
                packet.camera_fov * PI / 180.0,
                self.camera.aspect(),
                packet.camera_near,
                packet.camera_far,
            );
        }
    }

    /// Get a reference to the window for cursor control
    pub fn window(&self) -> &winit::window::Window {
        self.gfx.window()
    }
}

/// 鐎圭偟骞囩紒鐔剁閻ㄥ嫭瑕嗛弻鎾虫倵缁旑垱甯撮崣?
#[cfg(target_os = "windows")]
impl crate::renderer::backend_trait::RenderBackend for Renderer {
    fn window(&self) -> &winit::window::Window {
        self.window()
    }

    fn resize(&mut self) {
        self.resize()
    }

    fn draw(&mut self) -> crate::core::error::Result<()> {
        self.draw()
    }

    fn update(&mut self, input_system: &mut crate::core::input::InputSystem, delta_time: f32) {
        self.update(input_system, delta_time)
    }

    fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        self.apply_gui_packet(packet)
    }

    // handle_gui_event 娴ｈ法鏁ゆ妯款吇鐎圭偟骞囬敍鍫ｇ箲閸?false閿?
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            // Unmap 鐢悂鍣虹紓鎾冲暱閸?
            self.constant_buffer.Unmap(0, None);
            debug!("DX12 Renderer dropped, constant buffer unmapped");
        }
    }
}
