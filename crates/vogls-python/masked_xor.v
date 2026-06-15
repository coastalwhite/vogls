`timescale 1fs/1fs
module myxor();
    reg [31:0] a_0;
    reg [31:0] a_1;
    reg [31:0] b_0;
    reg [31:0] b_1;
    reg [31:0] z_0;
    reg [31:0] z_1;

    reg clk;
    reg [2:0] cnt;

    always begin clk = 0; #1 clk = 1; #1 ; end

    always @(posedge clk) begin
        if (&cnt) begin z_0 = a_0 ^ b_0; z_1 = a_1 ^ b_1; end
        else      begin z_0 = 0; z_1 = 0; end
        cnt = cnt + 1;
    end
endmodule
