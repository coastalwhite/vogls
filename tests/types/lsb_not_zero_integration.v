// vogls: verify-stdout
module repro (
    input clk,
    input load,
    output reg [7:0] out
);
    reg [7:1] q;              // non-zero LSB
    wire [6:0] q_next = q[7:1] + 7'd1;  // combinational read of q

    always @(posedge clk) begin
        if (load) q <= 7'h10;
        else      q <= q_next;
        out <= {q, 1'b0};
    end
endmodule

module tb();
    reg clk = 0;
    reg load = 1;
    wire [7:0] out;

    always #5 clk = ~clk;
    repro dut (.clk(clk), .load(load), .out(out));

    initial begin
        #10 load = 0;  // posedge: q <= 0x10
        #10;           // posedge: q <= 0x11, out <= {0x10, 0} = 0x20
        #10;           // posedge: q <= 0x12, out <= {0x11, 0} = 0x22
        // out should be 0x22
        if (out === 8'h22)
            $display("SUCCESS: out=%h", out);
        else
            $display("FAILURE: out=%h (expected 22)", out);
        $finish;
    end
endmodule
