`timescale 1 fs / 1 fs

// Configuration knobs, driven by macros defined from the Rust side.
`ifdef EXTENSION_M
`define EXTENSION_M 1
`else
`define EXTENSION_M 0
`endif
`ifdef MUL_FAST
`define MUL_FAST 1
`else
`define MUL_FAST 0
`endif
`ifdef MULH_FAST
`define MULH_FAST 1
`else
`define MULH_FAST 0
`endif
`ifdef MULDIV_UNROLL_2
`define MULDIV_UNROLL 2
`else
`define MULDIV_UNROLL 1
`endif
`ifdef REDUCED_BYPASS
`define REDUCED_BYPASS 1
`else
`define REDUCED_BYPASS 0
`endif
`ifdef BRANCH_PREDICTOR
`define BRANCH_PREDICTOR 1
`else
`define BRANCH_PREDICTOR 0
`endif
`ifdef FAST_BRANCHCMP
`define FAST_BRANCHCMP 1
`else
`define FAST_BRANCHCMP 0
`endif

module tb();

    reg clk = 0, rst_n = 0;

    // ------------------------------------------------------------------
    // AHB5-Lite manager port
    // ------------------------------------------------------------------
    wire [31:0] haddr;
    wire        hwrite;
    wire [1:0]  htrans;
    wire [2:0]  hsize;
    wire [2:0]  hburst;
    wire [3:0]  hprot;
    wire        hmastlock;
    wire [7:0]  hmaster;
    wire        hexcl;
    wire [31:0] hwdata;
    reg  [31:0] hrdata;

    wire        pwrup_req;
    wire        unblock_out;

    hazard3_cpu_1port #(
        .RESET_VECTOR        (32'h0000_0000),
        .MTVEC_INIT          (32'h0000_0000),

        // The assembler feeds us uncompressed RV32I(M), and the explorer maps
        // a PC onto an instruction slot by dividing by four, so the C
        // extension is left out.
        .EXTENSION_A         (0),
        .EXTENSION_C         (0),
        .EXTENSION_E         (0),
        .EXTENSION_M         (`EXTENSION_M),
        .EXTENSION_ZIFENCEI  (0),

        .CSR_M_MANDATORY     (1),
        .CSR_M_TRAP          (1),
        .CSR_COUNTER         (0),
        .U_MODE              (0),
        .PMP_REGIONS         (0),
        .DEBUG_SUPPORT       (0),
        .BREAKPOINT_TRIGGERS (0),
        .NUM_IRQS            (1),
        .IRQ_PRIORITY_BITS   (0),
        .RESET_REGFILE       (1),

        .REDUCED_BYPASS      (`REDUCED_BYPASS),
        .MULDIV_UNROLL       (`MULDIV_UNROLL),
        .MUL_FAST            (`EXTENSION_M && `MUL_FAST),
        .MUL_FASTER          (0),
        .MULH_FAST           (`EXTENSION_M && `MUL_FAST && `MULH_FAST),
        .FAST_BRANCHCMP      (`FAST_BRANCHCMP),
        .BRANCH_PREDICTOR    (`BRANCH_PREDICTOR)
    ) cpu (
        .clk                        (clk),
        .clk_always_on              (clk),
        .rst_n                      (rst_n),

        .pwrup_req                  (pwrup_req),
        .pwrup_ack                  (pwrup_req),   // Tied back
        .clk_en                     (),
        .unblock_out                (unblock_out),
        .unblock_in                 (unblock_out), // Tied back

        .haddr                      (haddr),
        .hwrite                     (hwrite),
        .htrans                     (htrans),
        .hsize                      (hsize),
        .hburst                     (hburst),
        .hprot                      (hprot),
        .hmastlock                  (hmastlock),
        .hmaster                    (hmaster),
        .hexcl                      (hexcl),
        .hready                     (1'b1),
        .hresp                      (1'b0),
        .hexokay                    (1'b1),
        .hwdata                     (hwdata),
        .hrdata                     (hrdata),

        .fence_i_vld                (),
        .fence_d_vld                (),
        .fence_rdy                  (1'b1),

        .dbg_req_halt               (1'b0),
        .dbg_req_halt_on_reset      (1'b0),
        .dbg_req_resume             (1'b0),
        .dbg_halted                 (),
        .dbg_running                (),

        .dbg_data0_rdata            (32'h0),
        .dbg_data0_wdata            (),
        .dbg_data0_wen              (),

        .dbg_instr_data             (32'h0),
        .dbg_instr_data_vld         (1'b0),
        .dbg_instr_data_rdy         (),
        .dbg_instr_caught_exception (),
        .dbg_instr_caught_ebreak    (),

        .dbg_sbus_addr              (32'h0),
        .dbg_sbus_write             (1'b0),
        .dbg_sbus_size              (2'h0),
        .dbg_sbus_vld               (1'b0),
        .dbg_sbus_rdy               (),
        .dbg_sbus_err               (),
        .dbg_sbus_wdata             (32'h0),
        .dbg_sbus_rdata             (),

        .mhartid_val                (32'h0),
        .eco_version                (4'h0),

        .irq                        (1'b0),
        .soft_irq                   (1'b0),
        .timer_irq                  (1'b0)
    );

    // ------------------------------------------------------------------
    // Memory
    //
    // `memory` holds the text section and is written from the host before the
    // simulation starts; `data_mem` backs everything from 1024 upwards.
    // ------------------------------------------------------------------
    reg [31:0] memory   [0:255];
    reg [31:0] data_mem [0:1000];

    // Clock generation
    always begin
        clk = 0;
        #1 clk = 1;
        #1 ;
    end

    // ------------------------------------------------------------------
    // AHB5-Lite subordinate: zero wait states, so the data phase of a
    // transfer is simply the cycle after its address phase.
    // ------------------------------------------------------------------
    reg [31:0] addr_dph;
    reg        write_dph;
    reg [2:0]  size_dph;
    reg        active_dph;

    always @(posedge clk) begin
        if (!rst_n) begin
            active_dph <= 1'b0;
            addr_dph   <= 32'h0;
            write_dph  <= 1'b0;
            size_dph   <= 3'h0;
        end else begin
            active_dph <= htrans[1];
            addr_dph   <= haddr;
            write_dph  <= hwrite;
            size_dph   <= hsize;
        end
    end

    always @(*) begin
        if (addr_dph < 1024)
            hrdata = memory[addr_dph[9:2]];
        else
            hrdata = data_mem[(addr_dph - 1024) >> 2];
    end

    // Byte lanes selected by the transfer size and the address offset. AHB
    // presents write data already aligned to its lanes.
    reg [3:0] wstrb;
    always @(*) begin
        case (size_dph[1:0])
            2'd0:    wstrb = 4'b0001 << addr_dph[1:0];
            2'd1:    wstrb = 4'b0011 << {addr_dph[1], 1'b0};
            default: wstrb = 4'b1111;
        endcase
    end

    always @(posedge clk) begin
        if (active_dph && write_dph) begin
            if (addr_dph < 1024) begin
                if (wstrb[0]) memory[addr_dph[9:2]][ 7: 0] <= hwdata[ 7: 0];
                if (wstrb[1]) memory[addr_dph[9:2]][15: 8] <= hwdata[15: 8];
                if (wstrb[2]) memory[addr_dph[9:2]][23:16] <= hwdata[23:16];
                if (wstrb[3]) memory[addr_dph[9:2]][31:24] <= hwdata[31:24];
            end else begin
                if (wstrb[0]) data_mem[(addr_dph - 1024) >> 2][ 7: 0] <= hwdata[ 7: 0];
                if (wstrb[1]) data_mem[(addr_dph - 1024) >> 2][15: 8] <= hwdata[15: 8];
                if (wstrb[2]) data_mem[(addr_dph - 1024) >> 2][23:16] <= hwdata[23:16];
                if (wstrb[3]) data_mem[(addr_dph - 1024) >> 2][31:24] <= hwdata[31:24];
            end
        end
    end

    // ------------------------------------------------------------------
    // Pipeline monitor
    //
    // Hazard3 is a three-stage pipeline: F (fetch), X (decode + execute) and
    // M (memory + writeback).
    //
    // F and X hold their instruction combinationally, so those are tapped
    // directly. M has no PC of its own in the synthesised core, so the X -> M
    // handoff is replicated here; the logic mirrors the `rvfm_xm_pc` /
    // `rvfm_m_valid` registers of hdl/hazard3_rvfi_monitor.vh.
    // ------------------------------------------------------------------

    // F: address phase of the instruction fetch.
    wire        f_active = cpu.core.frontend.mem_addr_vld;
    wire [31:0] f_pc     = cpu.core.frontend.mem_addr;

    // X: the frontend has an instruction ready for decode/execute.
    wire        x_active = !cpu.core.d_starved;
    wire [31:0] x_pc     = cpu.core.d_pc;

    wire        x_stall  = cpu.core.x_stall;
    wire        m_stall  = cpu.core.m_stall;
    wire        trap_now = cpu.core.m_trap_enter_vld && cpu.core.m_trap_enter_rdy;

    // M: whatever left X on the last cycle it was not stalled.
    reg         m_active;
    reg [31:0]  m_pc;

    always @(posedge clk) begin
        if (!rst_n) begin
            m_active <= 1'b0;
            m_pc     <= 32'h0;
        end else begin
            if (!x_stall) begin
                m_active <= |cpu.core.df_cir_use && !trap_now;
                m_pc     <= x_pc;
            end else if (!m_stall) begin
                m_active <= 1'b0;
            end
        end
    end

    // Latched so the host sees the trap on the cycle it samples.
    reg trap_q;
    always @(posedge clk) begin
        if (!rst_n)
            trap_q <= 1'b0;
        else
            trap_q <= trap_q || trap_now;
    end

endmodule
