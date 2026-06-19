`timescale 1 fs / 1 fs

module tb();
    reg clk = 0, rstn = 0;

    wire [31:0] xbus_adr;
    wire [31:0] xbus_wdata;
    wire        xbus_we;
    wire [3:0]  xbus_sel;
    wire        xbus_stb;
    wire        xbus_cyc;
    reg  [31:0] xbus_rdata;
    reg         xbus_ack;

    neorv32_minimal_wrapper neorv32_inst (
        .clk_i      (clk),
        .rstn_i     (rstn),
        .xbus_adr_o (xbus_adr),
        .xbus_dat_o (xbus_wdata),
        .xbus_we_o  (xbus_we),
        .xbus_sel_o (xbus_sel),
        .xbus_stb_o (xbus_stb),
        .xbus_cyc_o (xbus_cyc),
        .xbus_dat_i (xbus_rdata),
        .xbus_ack_i (xbus_ack),
        .trap_o     ()
    );

    reg  trap_q;
    always @(posedge clk)
        trap_q <= neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_trap] ;

    // IF stage: instruction bus request from frontend
    wire if_active = neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[stb] ;
    wire [31:0] if_pc = 32'h0 | neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[addr] ;

    // Execution-stage signals from ctrl_o (all combinational)
    wire is_active  = neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_ready] ;   // S_DISPATCH
    wire [31:0] pc  = 32'h0 | neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_cur] ;
    // cnt_event bits: [2]=S_EXECUTE, [5]=S_ALU_WAIT, [6]=S_BRANCH
    wire [10:0] cnt_event = 11'h0 | neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cnt_event] ;
    wire ex_active  = cnt_event[2];
    wire alu_active = cnt_event[5];
    wire br_active  = cnt_event[6];
    wire ma_active  = neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mo_en]
                    | neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mi_en] ;

    // Register all combinational signals so they are stable after posedge.
    // pc_cur during S_DISPATCH holds the *next* instruction's PC, so double-register
    // it to align with retired_q (which captures the just-dispatched instruction).
    reg retired_q, is_q, ex_q, alu_q, br_q, ma_q, if_q;
    reg [31:0] pc_d, pc_q, if_pc_q;
    always @(posedge clk) begin
        retired_q <= neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_ready] ;
        pc_d      <= 32'h0 | neorv32_inst.neorv32_top_inst.core_complex_gen_n1_neorv32_cpu_inst.\neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_cur] ;
        pc_q      <= pc_d;
        is_q      <= is_active;
        ex_q      <= ex_active;
        alu_q     <= alu_active;
        br_q      <= br_active;
        ma_q      <= ma_active;
        if_q      <= if_active;
        if_pc_q   <= if_pc;
    end

    always begin
        clk = 0;
        #1 clk = 1;
        #1 ;
    end

    reg [31:0] memory [0:255];
    reg [31:0] primes [0:1000];

    // Wishbone B4 classic: stb pulses one cycle, ack+rdata registered the next.
    // Both xbus_rdata and xbus_ack are driven together so data is valid on ack.
    always @(posedge clk) begin
        xbus_ack   <= 0;
        xbus_rdata <= 32'h0;
        if (xbus_cyc && xbus_stb) begin
            xbus_ack <= 1;
            if (xbus_adr < 1024) begin
                xbus_rdata <= memory[xbus_adr >> 2];
                if (xbus_we) begin
                    if (xbus_sel[0]) memory[xbus_adr >> 2][ 7: 0] <= xbus_wdata[ 7: 0];
                    if (xbus_sel[1]) memory[xbus_adr >> 2][15: 8] <= xbus_wdata[15: 8];
                    if (xbus_sel[2]) memory[xbus_adr >> 2][23:16] <= xbus_wdata[23:16];
                    if (xbus_sel[3]) memory[xbus_adr >> 2][31:24] <= xbus_wdata[31:24];
                end
            end else begin
                xbus_rdata <= primes[(xbus_adr - 1024) >> 2];
                if (xbus_we) begin
                    if (xbus_sel[0]) primes[(xbus_adr - 1024) >> 2][ 7: 0] <= xbus_wdata[ 7: 0];
                    if (xbus_sel[1]) primes[(xbus_adr - 1024) >> 2][15: 8] <= xbus_wdata[15: 8];
                    if (xbus_sel[2]) primes[(xbus_adr - 1024) >> 2][23:16] <= xbus_wdata[23:16];
                    if (xbus_sel[3]) primes[(xbus_adr - 1024) >> 2][31:24] <= xbus_wdata[31:24];
                end
            end
        end
    end

endmodule
