`timescale 1fs/1fs
module myxor();
    reg [31:0] a;
    reg [31:0] b;
    reg [31:0] z;
    reg clk;
    reg [2:0] cnt;

    always begin clk = 0; #1 clk = 1; #1 ; end

    always @(posedge clk) begin
        if (&cnt) z = a ^ b;
        else      z = 0;
        cnt = cnt + 1;
    end
endmodule