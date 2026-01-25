use std::sync::Arc;
use std::mem::ManuallyDrop;
use winit::event_loop::EventLoop;
use crate::gfx::Dx12Backend;
use windows::Win32::Graphics::Dxgi::{IDXGISwapChain3, DXGI_PRESENT, Common::*};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::System::Threading::WaitForSingleObject;

#[repr(C)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

pub struct Renderer {
    gfx: Dx12Backend,
    root_signature: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    vertex_buffer: ID3D12Resource,
    vertex_buffer_view: D3D12_VERTEX_BUFFER_VIEW,
    viewport: D3D12_VIEWPORT,
    scissor_rect: RECT,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = winit::window::WindowBuilder::new()
            .build(event_loop)
            .unwrap();
        let window = Arc::new(window);
        
        let gfx = Dx12Backend::new(window);

        unsafe {
            // 1. Root Signature
            // let mut root_signature: Option<ID3D12RootSignature> = None; <--- Removed this line
            let root_desc = D3D12_ROOT_SIGNATURE_DESC {
                Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
                ..Default::default()
            };
            
            let mut signature = None;
            D3D12SerializeRootSignature(&root_desc, D3D_ROOT_SIGNATURE_VERSION_1, &mut signature, None).unwrap();
            let signature = signature.unwrap();
            
            let root_signature: ID3D12RootSignature = gfx.device.CreateRootSignature(0, std::slice::from_raw_parts(signature.GetBufferPointer() as _, signature.GetBufferSize())).unwrap();

            // 2. Shaders
            let shaders_hlsl = "
                struct PSInput {
                    float4 position : SV_POSITION;
                    float4 color : COLOR;
                };
                PSInput VSMain(float2 position : POSITION, float3 color : COLOR) {
                    PSInput result;
                    result.position = float4(position, 0.0, 1.0);
                    result.color = float4(color, 1.0);
                    return result;
                }
                float4 PSMain(PSInput input) : SV_TARGET {
                    return input.color;
                }
            ";

            let mut vs_blob = None;
            let mut ps_blob = None;
            let mut error_blob = None;

            let result = D3DCompile(
                shaders_hlsl.as_ptr() as _,
                shaders_hlsl.len(),
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
                shaders_hlsl.as_ptr() as _,
                shaders_hlsl.len(),
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

            // 3. Input Layout
            let input_element_descs = [
                D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: windows::core::s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D12_INPUT_ELEMENT_DESC {
                    SemanticName: windows::core::s!("COLOR"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 8,
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
                CullMode: D3D12_CULL_MODE_NONE,
                ..Default::default()
            };
            pso_desc.InputLayout = D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: input_element_descs.as_ptr(),
                NumElements: input_element_descs.len() as u32,
            };
            pso_desc.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
            pso_desc.NumRenderTargets = 1;
            pso_desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
            pso_desc.SampleDesc.Count = 1;

            let pso: ID3D12PipelineState = gfx.device.CreateGraphicsPipelineState(&pso_desc).expect("Failed to create PSO");

            // 5. Vertex Buffer
            let vertices = [
                Vertex { position: [0.0, 0.5], color: [1.0, 0.0, 0.0] },
                Vertex { position: [0.5, -0.5], color: [0.0, 1.0, 0.0] },
                Vertex { position: [-0.5, -0.5], color: [0.0, 0.0, 1.0] },
            ];
            let vertex_data_size = (std::mem::size_of::<Vertex>() * vertices.len()) as u64;

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
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), data as *mut Vertex, vertices.len());
            vertex_buffer.Unmap(0, None);

            let vertex_buffer_view = D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: vertex_buffer.GetGPUVirtualAddress(),
                SizeInBytes: vertex_data_size as u32,
                StrideInBytes: std::mem::size_of::<Vertex>() as u32,
            };

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

            Self {
                gfx,
                root_signature,
                pso,
                vertex_buffer,
                vertex_buffer_view,
                viewport,
                scissor_rect,
            }
        }
    }

    pub fn resize(&mut self) {
        // Handle swapchain resize
    }

    pub fn draw(&mut self) {
        unsafe {
            // Simple Clear Screen
            let command_allocator: ID3D12CommandAllocator = self.gfx.device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT).unwrap();
            let command_list: ID3D12GraphicsCommandList = self.gfx.device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None).unwrap();
            
            // Transition Barrier Present -> RenderTarget
            let barrier = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(Some(self.gfx.swap_chain.GetBuffer(self.gfx.frame_index as u32).unwrap())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_PRESENT,
                        StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET,
                    }),
                },
            };
            command_list.ResourceBarrier(&[barrier]);

            // Clear RTV
            let rtv_handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.gfx.rtv_heap.GetCPUDescriptorHandleForHeapStart().ptr + (self.gfx.frame_index * self.gfx.rtv_descriptor_size),
            };
            let clear_color = [0.0, 0.0, 0.2, 1.0]; // Dark Blue to distinguish
            command_list.ClearRenderTargetView(rtv_handle, &clear_color, None);

            // Draw Triangle
            command_list.SetGraphicsRootSignature(&self.root_signature);
            command_list.SetPipelineState(&self.pso);
            command_list.RSSetViewports(&[self.viewport]);
            command_list.RSSetScissorRects(&[self.scissor_rect]);
            
            command_list.OMSetRenderTargets(1, Some(&rtv_handle), false, None);
            command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            command_list.IASetVertexBuffers(0, Some(&[self.vertex_buffer_view]));
            command_list.DrawInstanced(3, 1, 0, 0);

            // Transition Barrier RenderTarget -> Present
            let barrier_back = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: ManuallyDrop::new(Some(self.gfx.swap_chain.GetBuffer(self.gfx.frame_index as u32).unwrap())),
                        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        StateBefore: D3D12_RESOURCE_STATE_RENDER_TARGET,
                        StateAfter: D3D12_RESOURCE_STATE_PRESENT,
                    }),
                },
            };
            command_list.ResourceBarrier(&[barrier_back]);

            command_list.Close().unwrap();

            // Execute
            let command_lists = [Some(ID3D12CommandList::from(command_list))];
            self.gfx.command_queue.ExecuteCommandLists(&command_lists);

            // Present
            self.gfx.swap_chain.Present(1, DXGI_PRESENT(0)).unwrap();

            // Wait for GPU (Brute force sync for simplicity)
            let fence_value = self.gfx.fence_value;
            self.gfx.command_queue.Signal(&self.gfx.fence, fence_value).unwrap();
            self.gfx.fence_value += 1;

            if self.gfx.fence.GetCompletedValue() < fence_value {
                self.gfx.fence.SetEventOnCompletion(fence_value, self.gfx.fence_event).unwrap();
                WaitForSingleObject(self.gfx.fence_event, windows::Win32::System::Threading::INFINITE);
            }

            self.gfx.frame_index = self.gfx.swap_chain.GetCurrentBackBufferIndex() as usize;
        }
    }
}
