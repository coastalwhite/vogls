`timescale 1 ns / 1 ps

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
		.ENABLE_MUL(1),
		.ENABLE_DIV(1)
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

        // Pico Co-Processor Interface (PCPI)
        .pcpi_valid(pcpi_valid),
        .pcpi_insn (pcpi_insn),
        .pcpi_rs1  (pcpi_rs1),
        .pcpi_rs2  (pcpi_rs2),
        .pcpi_wr   (pcpi_wr),
        .pcpi_rd   (pcpi_rd),
        .pcpi_wait (pcpi_wait),
        .pcpi_ready(pcpi_ready),

        // IRQ Interface
        .irq(irq),
        .eoi(eoi),

        // Trace Interface
        .trace_valid(trace_valid),
        .trace_data(trace_data)
    );

	reg [31:0] memory [0:255];
	reg [31:0] primes [0:1000];

    always begin
        #5
        clk = ~clk;
    end

    // initial begin
    //   $dumpfile("dump-icarus.vcd");
    //   $dumpvars;
    // end

    integer i;
	initial begin
        repeat (5) @(posedge clk);
		resetn = 1;
		repeat (100_000_000) @(posedge clk);
		for (i = 0; i < 10; i = i + 1) $display("Prime[%d] = %d", i, primes[i]);
		$display("hi!");
		$finish;
	end

	initial begin
		repeat (100) @(posedge clk);
		wait (trap) ;
		$display("Trap Detected!");
		$display("Prime = %d", primes[999]);
		$finish;
	end

	initial $readmemh("prime.hex", memory);

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

    // always @(trap) begin
    //     $display("trap");
    // end
 //    always @(posedge clk) begin
	// 	if (mem_valid && mem_ready) begin
	// 		if (mem_instr)
	// 			$display("ifetch 0x%08x: 0x%08x", mem_addr, mem_rdata);
	// 		else if (mem_wstrb)
	// 			$display("write  0x%08x: 0x%08x (wstrb=%b)", mem_addr, mem_wdata, mem_wstrb);
	// 		else
	// 			$display("read   0x%08x: 0x%08x", mem_addr, mem_rdata);
	// 	end
	// end
endmodule