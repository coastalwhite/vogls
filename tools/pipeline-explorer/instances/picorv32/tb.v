`timescale 1 fs / 1 fs

`ifdef ENABLE_MUL
`define ENABLE_MUL 1
`else
`define ENABLE_MUL 0
`endif
`ifdef ENABLE_DIV
`define ENABLE_DIV 1
`else
`define ENABLE_DIV 0
`endif
`ifdef TWO_STAGE_SHIFT
`define TWO_STAGE_SHIFT 1
`else
`define TWO_STAGE_SHIFT 0
`endif
`ifdef BARREL_SHIFTER
`define BARREL_SHIFTER 1
`else
`define BARREL_SHIFTER 0
`endif
`ifdef TWO_CYCLE_COMPARE
`define TWO_CYCLE_COMPARE 1
`else
`define TWO_CYCLE_COMPARE 0
`endif
`ifdef TWO_CYCLE_ALU
`define TWO_CYCLE_ALU 1
`else
`define TWO_CYCLE_ALU 0
`endif
`ifdef ENABLE_FAST_MUL
`define ENABLE_FAST_MUL 1
`else
`define ENABLE_FAST_MUL 0
`endif

module tb();
    reg clk = 0, resetn = 0;
    wire trap;

    wire mem_valid, mem_instr;
    reg mem_ready;

    wire [31:0] mem_addr, mem_wdata;
    wire [3:0]  mem_wstrb;
    reg [31:0] mem_rdata;

    wire mem_la_read, mem_la_write;
    wire [31:0] mem_la_addr, mem_la_wdata;
    wire [3:0] mem_la_wstrb;

    wire        pcpi_valid;
    wire [31:0] pcpi_insn;
    wire [31:0] pcpi_rs1;
    wire [31:0] pcpi_rs2;
    reg        pcpi_wr;
    reg [31:0] pcpi_rd;
    reg        pcpi_wait;
    reg        pcpi_ready;

    reg      [31:0] irq;
    wire [31:0] eoi;

    wire        trace_valid;
    wire [35:0] trace_data;
    
    picorv32 #(
		.ENABLE_MUL(`ENABLE_MUL),
		.ENABLE_DIV(`ENABLE_DIV),
        .TWO_STAGE_SHIFT(`TWO_STAGE_SHIFT),
        .BARREL_SHIFTER(`BARREL_SHIFTER),
        .TWO_CYCLE_COMPARE(`TWO_CYCLE_COMPARE),
        .TWO_CYCLE_ALU(`TWO_CYCLE_ALU),
        .ENABLE_FAST_MUL(`ENABLE_FAST_MUL)
	) proc (

        .clk(clk), .resetn(resetn),
        .trap(trap),

        .mem_valid(mem_valid),
        .mem_instr(mem_instr),
        .mem_ready(mem_ready),

        .mem_addr (mem_addr),
        .mem_wdata(mem_wdata),
        .mem_wstrb(mem_wstrb),
        .mem_rdata(mem_rdata),

        // Look-Ahead Interface
        .mem_la_read (mem_la_read ),
        .mem_la_write(mem_la_write),
        .mem_la_addr (mem_la_addr ),
        .mem_la_wdata(mem_la_wdata),
        .mem_la_wstrb(mem_la_wstrb),

        .pcpi_valid(pcpi_valid),
        .pcpi_insn (pcpi_insn),
        .pcpi_rs1  (pcpi_rs1),
        .pcpi_rs2  (pcpi_rs2),
        .pcpi_wr   (pcpi_wr),
        .pcpi_rd   (pcpi_rd),
        .pcpi_wait (pcpi_wait),
        .pcpi_ready(pcpi_ready),

        .irq(irq),
        .eoi(eoi),

        // Trace Interface
        .trace_valid(trace_valid),
        .trace_data(trace_data)
    );

	reg [31:0] memory [0:255];
	reg [31:0] primes [0:1000];

    always begin
        clk = 0;
        #1 clk = 1;
        #1 ;
    end

	initial begin
		repeat (100) @(posedge clk);
		wait (trap) ;
		$display("Trap Detected!");
		$display("Prime = %d", primes[999]);
		$finish;
	end

	always @(posedge clk) begin
		mem_ready <= 0;
		if (mem_valid && !mem_ready) begin
			if (mem_addr < 1024) begin
				mem_ready <= 1;
				mem_rdata <= memory[mem_addr >> 2];
				if (mem_wstrb[0]) memory[mem_addr >> 2][ 7: 0] <= mem_wdata[ 7: 0];
				if (mem_wstrb[1]) memory[mem_addr >> 2][15: 8] <= mem_wdata[15: 8];
				if (mem_wstrb[2]) memory[mem_addr >> 2][23:16] <= mem_wdata[23:16];
				if (mem_wstrb[3]) memory[mem_addr >> 2][31:24] <= mem_wdata[31:24];
			end

            if (mem_addr >= 1024) begin
				mem_ready <= 1;
				mem_rdata <= primes[(mem_addr - 1024) >> 2];
				if (mem_wstrb[0]) primes[(mem_addr - 1024) >> 2][ 7: 0] <= mem_wdata[ 7: 0];
				if (mem_wstrb[1]) primes[(mem_addr - 1024) >> 2][15: 8] <= mem_wdata[15: 8];
				if (mem_wstrb[2]) primes[(mem_addr - 1024) >> 2][23:16] <= mem_wdata[23:16];
				if (mem_wstrb[3]) primes[(mem_addr - 1024) >> 2][31:24] <= mem_wdata[31:24];
            end
		end
	end
endmodule