// vogls: include-dir=include
`include "in-include-dir.v"

module tb();
    wire [15:0] z;
    xyz _x(z);
    initial #0 $vogls_assert_eq(z, 16'h1337);
endmodule
