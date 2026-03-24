// vogls: verify-ir
module tb();
    reg a, b;
    wire [1:0] c;

    assign c[0] = a;
    assign c[1] = b;

`ifndef __VOGLS_VERIFY_IR
    initial begin
        a = 0; b = 0; #0 $vogls_assert_eq(c, 2'b00);
        a = 1; b = 0; #0 $vogls_assert_eq(c, 2'b01);
        a = 0; b = 1; #0 $vogls_assert_eq(c, 2'b10);
        a = 1; b = 1; #0 $vogls_assert_eq(c, 2'b11);
    end
`endif
endmodule
