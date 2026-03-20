// vogls: verify-ir
module bufmod(
    input i,
    output o
);
    assign o = i;
endmodule

module top2 ();
    reg a;
    wire [1:0] z;

    bufmod _b0 ( .i(a[0]), .o(z[0]) );

`ifndef __VOGLS_VERIFY_IR
    initial begin
        z = 2'b0;
        #0 a = 1'b0; #0 $vogls_assert_eq(z, 2'b0);
        #0 a = 1'b1; #0 $vogls_assert_eq(z, 2'b1);
    end
`endif
endmodule
