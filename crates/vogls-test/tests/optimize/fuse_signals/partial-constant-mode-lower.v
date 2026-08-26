// vogls: mode=two-value-logic
module top;
    wire [31:0] x_net;
    reg  [31:0] out;

    assign x_net = 32'bx;

    always @* begin
        out = 32'b0;
        if (1'b1) out = x_net;
    end

    initial begin
        #1;
        $vogls_assert_eq(out, 32'b0);
    end
endmodule
