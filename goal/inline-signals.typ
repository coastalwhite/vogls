= Inline Signals Optimization Pass

The inline signals optimization pass aims to remove processes that
unconditionally propagate signals expressions to other signals. This
complements the _fuse signals_ pass in that it can combine processes for more
complex expressions.

This optimization pass is especially important for gate-level simulations where
things like the following occur frequently.

```verilog
module SB_LUT4 (
  output O,
  input I0, I1, I2, I3
);
  parameter [15:0] LUT_INIT = 0;
  wire [7:0] s3 = I3 ? LUT_INIT[15:8] : LUT_INIT[7:0];
  wire [3:0] s2 = I2 ?       s3[ 7:4] :       s3[3:0];
  wire [1:0] s1 = I1 ?       s2[ 3:2] :       s2[1:0];
  assign O = I0 ? s1[1] : s1[0];
endmodule
```

The desired outcome after the optimization pass would be one process. Note
there is some peephole optimization going on as well.

```llvm
proc LUT4 {
entry:
    %i3  = prb $I3
    %i3i = zero_extend[32] %i3
    %i3o = slli %i3i, 3
    %t0 = revslicezi %i3o, LUT_INIT
    // [if-observed] drv $s3, %t0

    %i2 = prb $I2
    %i2i = zero_extend[32] %i2
    %i2o = slli %i2i, 2
    %t1 = slicez %t0, %i2o
    // [if-observed] drv $s2, %t0

    %i1 = prb $I1
    %i1i = zero_extend[32] %i1
    %i1o = slli %i1i, 1
    %t2 = slicez %t1, %i1o
    // [if-observed] drv $s1, %t0

    %i0 = prb $I0
    %i0i = zero_extend[32] %i0
    %t3 = slicez %t2, %i0o

    drv $O, %t3
    watch [I3, I2, I1, I0], <entry>
}
```
