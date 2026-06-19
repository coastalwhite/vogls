`timescale 1 fs / 1 fs

module BUFGCE(input I, input CE, output O);
    parameter SIM_DEVICE = "7SERIES";
    assign O = I & CE;
endmodule

module tb();

    reg clk = 0, rst_n = 0;

    // Instruction memory interface
    wire        instr_req;
    reg         instr_gnt;
    reg         instr_rvalid;
    wire [31:0] instr_addr;
    reg  [31:0] instr_rdata;

    // Data memory interface
    wire        data_req;
    reg         data_gnt;
    reg         data_rvalid;
    wire        data_we;
    wire [3:0]  data_be;
    wire [31:0] data_addr;
    wire [31:0] data_wdata;
    reg  [31:0] data_rdata;

    // Alert / sleep outputs (unused)
    wire        alert_minor, alert_major_internal, alert_major_bus, core_sleep;

    ibex_top #(
        .PMPEnable        ( 0                ),
        .PMPGranularity   ( 0                ),
        .PMPNumRegions    ( 4                ),
        .MHPMCounterNum   ( 0                ),
        .MHPMCounterWidth ( 40               ),
        .RV32E            ( 0                ),
        .RV32M            ( 2                ),
        .RV32B            ( 0                ),
        .RV32ZC           ( 0                ),
        .RegFile          ( 0                ),
        .ICache           ( 0                ),
        .ICacheECC        ( 0                ),
        .ICacheScramble   ( 0                ),
        .BranchPredictor  ( 0                ),
`ifdef WRITEBACK_STAGE
        .WritebackStage   ( 1'b1             ),
`endif
        .SecureIbex       ( 0                ),
        .RndCnstLfsrSeed  ( 0                ),
        .RndCnstLfsrPerm  ( 0                ),
        .DbgTriggerEn     ( 0                ),
        .DmBaseAddr       ( 32'h1A110000     ),
        .DmAddrMask       ( 32'h00000FFF     ),
        .DmHaltAddr       ( 32'h1A110800     ),
        .DmExceptionAddr  ( 32'h1A110808     )
    ) u_top (
        // Clock and reset
        .clk_i                     (clk),
        .rst_ni                    (rst_n),
        .test_en_i                 (1'b0),
        .scan_rst_ni               (1'b1),
        .ram_cfg_icache_tag_i      (12'h0),
        .ram_cfg_rsp_icache_tag_o  (),
        .ram_cfg_icache_data_i     (12'h0),
        .ram_cfg_rsp_icache_data_o (),

        // Configuration
        .hart_id_i              (32'h0),
        .boot_addr_i            (32'h0),

        // Instruction memory interface
        .instr_req_o            (instr_req),
        .instr_gnt_i            (instr_gnt),
        .instr_rvalid_i         (instr_rvalid),
        .instr_addr_o           (instr_addr),
        .instr_rdata_i          (instr_rdata),
        .instr_rdata_intg_i     (7'h0),
        .instr_err_i            (1'b0),

        // Data memory interface
        .data_req_o             (data_req),
        .data_gnt_i             (data_gnt),
        .data_rvalid_i          (data_rvalid),
        .data_we_o              (data_we),
        .data_be_o              (data_be),
        .data_addr_o            (data_addr),
        .data_wdata_o           (data_wdata),
        .data_wdata_intg_o      (),
        .data_rdata_i           (data_rdata),
        .data_rdata_intg_i      (7'h0),
        .data_err_i             (1'b0),

        // Interrupt inputs
        .irq_software_i         (1'b0),
        .irq_timer_i            (1'b0),
        .irq_external_i         (1'b0),
        .irq_fast_i             (15'h0),
        .irq_nm_i               (1'b0),

        // Debug interface
        .debug_req_i            (1'b0),
        .crash_dump_o           (),

        // Special control signals
        .fetch_enable_i         (4'b0101),
        .alert_minor_o          (alert_minor),
        .alert_major_internal_o (alert_major_internal),
        .alert_major_bus_o      (alert_major_bus),
        .core_sleep_o           (core_sleep)
    );

    reg [31:0] memory [0:255];
    reg [31:0] primes [0:1000];

    // Clock generation
    always begin
        clk = 0;
        #1 clk = 1;
        #1 ;
    end

    reg [31:0] instr_addr_q;

    // Grant immediately
    always @(*) begin
        instr_gnt = instr_req;
    end

    // Latch the accepted address, drive rvalid one cycle later
    always @(posedge clk) begin
        if (!rst_n) begin
            instr_rvalid <= 0;
            instr_addr_q <= 0;
        end else begin
            instr_rvalid <= instr_req & instr_gnt;
            if (instr_req & instr_gnt)
                instr_addr_q <= instr_addr;
        end
    end

    // rdata is combinational from the latched address, valid when rvalid is high
    always @(*) begin
        if (instr_addr_q < 1024)
            instr_rdata = memory[instr_addr_q >> 2];
        else
            instr_rdata = primes[(instr_addr_q - 1024) >> 2];
    end

    // ------------------------------------------------------------------
    // Data memory
    // OBI-like: grant is combinational (same cycle as req),
    //           rvalid + rdata appear one cycle after grant.
    //           Writes take effect on the rvalid cycle.
    // ------------------------------------------------------------------
    reg [31:0] data_addr_q;
    reg        data_we_q;
    reg [3:0]  data_be_q;
    reg [31:0] data_wdata_q;

    // Grant immediately
    always @(*) begin
        data_gnt = data_req;
    end

    // Latch accepted transaction, drive rvalid one cycle later
    always @(posedge clk) begin
        if (!rst_n) begin
            data_rvalid  <= 0;
            data_addr_q  <= 0;
            data_we_q    <= 0;
            data_be_q    <= 0;
            data_wdata_q <= 0;
        end else begin
            data_rvalid <= data_req & data_gnt;
            if (data_req & data_gnt) begin
                data_addr_q  <= data_addr;
                data_we_q    <= data_we;
                data_be_q    <= data_be;
                data_wdata_q <= data_wdata;
            end
        end
    end

    // rdata is combinational from the latched address
    always @(*) begin
        if (data_addr_q < 1024)
            data_rdata = memory[data_addr_q >> 2];
        else
            data_rdata = primes[(data_addr_q - 1024) >> 2];
    end

    // Writes happen on the rvalid cycle from latched signals
    always @(posedge clk) begin
        if (data_rvalid && data_we_q) begin
            if (data_addr_q < 1024) begin
                if (data_be_q[0]) memory[data_addr_q >> 2][ 7: 0] <= data_wdata_q[ 7: 0];
                if (data_be_q[1]) memory[data_addr_q >> 2][15: 8] <= data_wdata_q[15: 8];
                if (data_be_q[2]) memory[data_addr_q >> 2][23:16] <= data_wdata_q[23:16];
                if (data_be_q[3]) memory[data_addr_q >> 2][31:24] <= data_wdata_q[31:24];
            end else begin
                if (data_be_q[0]) primes[(data_addr_q - 1024) >> 2][ 7: 0] <= data_wdata_q[ 7: 0];
                if (data_be_q[1]) primes[(data_addr_q - 1024) >> 2][15: 8] <= data_wdata_q[15: 8];
                if (data_be_q[2]) primes[(data_addr_q - 1024) >> 2][23:16] <= data_wdata_q[23:16];
                if (data_be_q[3]) primes[(data_addr_q - 1024) >> 2][31:24] <= data_wdata_q[31:24];
            end
        end
    end

endmodule
