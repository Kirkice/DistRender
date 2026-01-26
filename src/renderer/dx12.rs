use std::sync::Arc;
use std::mem::ManuallyDrop;
use tracing::{trace, debug, info, warn, error};
use winit::event_loop::EventLoop;
use crate::gfx::Dx12Backend;
use crate::core::Config;
use crate::core::math::{Vector2, Vector3};
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_SWAP_CHAIN_FLAG, Common::*};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Threading::WaitForSingleObject;

#[repr(C)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    /// 从数学库的 Vector 类型创建顶点
    fn from_vectors(position: Vector2, color: Vector3) -> Self {
        Self {
            position: [position.x, position.y],
            color: [color.x, color.y, color.z],
        }
    }
}

const FRAME_COUNT: usize = 2;

pub struct Renderer {
    gfx: Dx12Backend,
    root_signature: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    #[allow(dead_code)]  // 保留供将来使用
    vertex_buffer: ID3D12Resource,
    vertex_buffer_view: D3D12_VERTEX_BUFFER_VIEW,
    viewport: D3D12_VIEWPORT,
    scissor_rect: RECT,
    command_allocators: [ID3D12CommandAllocator; FRAME_COUNT],
    command_list: ID3D12GraphicsCommandList,
    fence_values: [u64; FRAME_COUNT],
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        let gfx = Dx12Backend::new(event_loop, config);

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

            // 2. Shaders（从外部文件读取 HLSL）
            use std::fs;
            let hlsl_path = "src/renderer/shaders/shader.hlsl";
            let shaders_hlsl = fs::read_to_string(hlsl_path)
                .expect("Failed to read shader.hlsl");

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
            // 显式禁用深度测试（因为我们没有深度缓冲区）
            pso_desc.DepthStencilState = D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: false.into(),
                DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
                DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                StencilEnable: false.into(),
                StencilReadMask: 0,
                StencilWriteMask: 0,
                FrontFace: D3D12_DEPTH_STENCILOP_DESC::default(),
                BackFace: D3D12_DEPTH_STENCILOP_DESC::default(),
            };
            pso_desc.SampleMask = 0xFFFFFFFF;
            pso_desc.DSVFormat = DXGI_FORMAT_UNKNOWN;
            pso_desc.InputLayout = D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: input_element_descs.as_ptr(),
                NumElements: input_element_descs.len() as u32,
            };
            pso_desc.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
            pso_desc.NumRenderTargets = 1;
            pso_desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
            pso_desc.SampleDesc.Count = 1;

            let pso: ID3D12PipelineState = gfx.device.CreateGraphicsPipelineState(&pso_desc).expect("Failed to create PSO");

            // 5. Vertex Buffer - 使用数学库类型创建顶点数据
            let vertices = [
                Vertex::from_vectors(
                    Vector2::new(0.0, 0.5),
                    Vector3::new(1.0, 0.0, 0.0)  // 红色
                ),
                Vertex::from_vectors(
                    Vector2::new(0.5, -0.5),
                    Vector3::new(0.0, 1.0, 0.0)  // 绿色
                ),
                Vertex::from_vectors(
                    Vector2::new(-0.5, -0.5),
                    Vector3::new(0.0, 0.0, 1.0)  // 蓝色
                ),
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

            // 7. 创建命令对象（双缓冲）
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

            // 初始创建时命令列表是打开状态，需要先关闭
            command_list.Close().expect("Failed to close initial CommandList");

            #[cfg(debug_assertions)]
            info!("DX12 Renderer initialized successfully");

            Self {
                gfx,
                root_signature,
                pso,
                vertex_buffer,
                vertex_buffer_view,
                viewport,
                scissor_rect,
                command_allocators,
                command_list,
                fence_values: [0; FRAME_COUNT],
            }
        }
    }

    pub fn resize(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            debug!("Resizing swapchain...");

            // 等待 GPU 完成所有工作
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

            // 获取新的窗口大小
            let size = self.gfx.window.inner_size();
            self.gfx.width = size.width;
            self.gfx.height = size.height;

            // 调整交换链大小（会自动释放旧的缓冲区）
            self.gfx.swap_chain.ResizeBuffers(
                FRAME_COUNT as u32,
                size.width,
                size.height,
                DXGI_FORMAT_R8G8B8A8_UNORM,
                DXGI_SWAP_CHAIN_FLAG(0),
            ).expect("Failed to resize swap chain buffers");

            // 重新创建 RTV
            let rtv_handle = self.gfx.rtv_heap.GetCPUDescriptorHandleForHeapStart();
            for i in 0..FRAME_COUNT {
                let surface: ID3D12Resource = self.gfx.swap_chain.GetBuffer(i as u32)
                    .expect("Failed to get swap chain buffer");
                let handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + (i * self.gfx.rtv_descriptor_size),
                };
                self.gfx.device.CreateRenderTargetView(&surface, None, handle);
            }

            // 更新 viewport 和 scissor rect
            self.viewport.Width = size.width as f32;
            self.viewport.Height = size.height as f32;
            self.scissor_rect.right = size.width as i32;
            self.scissor_rect.bottom = size.height as i32;

            // 重置 frame index
            self.gfx.frame_index = self.gfx.swap_chain.GetCurrentBackBufferIndex() as usize;

            // 清除 fence 值（因为我们等待了所有帧完成）
            self.fence_values = [0; FRAME_COUNT];

            #[cfg(debug_assertions)]
            debug!(width = size.width, height = size.height, "Resize completed");
        }
    }

    pub fn draw(&mut self) {
        unsafe {
            let frame_index = self.gfx.frame_index;

            #[cfg(debug_assertions)]
            {
                let completed_value = self.gfx.fence.GetCompletedValue();
                trace!(frame_index, fence_value = self.gfx.fence_value, completed = completed_value, "Frame state");
            }

            // 只等待当前帧的前一次使用完成（双缓冲优化）
            let fence_value = self.fence_values[frame_index];
            if fence_value > 0 && self.gfx.fence.GetCompletedValue() < fence_value {
                #[cfg(debug_assertions)]
                debug!(frame_index, fence_value, "Waiting for GPU");

                self.gfx.fence.SetEventOnCompletion(fence_value, self.gfx.fence_event)
                    .expect("Failed to set fence event");
                WaitForSingleObject(self.gfx.fence_event, windows::Win32::System::Threading::INFINITE);

                #[cfg(debug_assertions)]
                debug!(frame_index, "GPU wait completed");
            }

            // 重置当前帧的命令分配器和命令列表
            let allocator = &self.command_allocators[frame_index];
            allocator.Reset().expect("Failed to reset CommandAllocator");
            self.command_list.Reset(allocator, Some(&self.pso))
                .expect("Failed to reset CommandList");

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
            self.command_list.ResourceBarrier(&[barrier]);

            // Clear RTV
            let rtv_handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.gfx.rtv_heap.GetCPUDescriptorHandleForHeapStart().ptr + (self.gfx.frame_index * self.gfx.rtv_descriptor_size),
            };
            let clear_color = [0.0, 0.0, 0.2, 1.0]; // Dark Blue to distinguish
            self.command_list.ClearRenderTargetView(rtv_handle, &clear_color, None);

            // Draw Triangle
            self.command_list.SetGraphicsRootSignature(&self.root_signature);
            self.command_list.SetPipelineState(&self.pso);
            self.command_list.RSSetViewports(&[self.viewport]);
            self.command_list.RSSetScissorRects(&[self.scissor_rect]);
            
            self.command_list.OMSetRenderTargets(1, Some(&rtv_handle), false, None);
            self.command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.command_list.IASetVertexBuffers(0, Some(&[self.vertex_buffer_view]));
            self.command_list.DrawInstanced(3, 1, 0, 0);

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
            self.command_list.ResourceBarrier(&[barrier_back]);

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

            // Signal fence and store the value for this frame
            let next_fence_value = self.gfx.fence_value;
            self.gfx.command_queue.Signal(&self.gfx.fence, next_fence_value)
                .expect("Failed to signal fence");
            self.fence_values[frame_index] = next_fence_value;
            self.gfx.fence_value += 1;

            // Update frame index
            self.gfx.frame_index = self.gfx.swap_chain.GetCurrentBackBufferIndex() as usize;

            #[cfg(debug_assertions)]
            trace!(frame_index, next_frame = self.gfx.frame_index, "Frame completed");
        }
    }
}
