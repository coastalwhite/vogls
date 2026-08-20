// vogls: mode=four-value-logic
`timescale 1ns/1ps

module tb;
  time x;
  integer y;
  realtime z;
  real w;
  reg [63:0] b;


  initial begin
    $vogls_assert_eq(x, 64'bx);
    $vogls_assert_eq(y, 32'bx);
    $vogls_assert_eq(z, 0.0);
    $vogls_assert_eq(w, 0.0);

    x = 64'h1_0000_0000;
    $vogls_assert_eq(x, 64'h0000_0001_0000_0000);
    y = 64'h1_0000_0000;
    $vogls_assert_eq(y, 32'h0000_0000);

    x = -1;
    $vogls_assert_eq(x, 64'hFFFF_FFFF_FFFF_FFFF);
    $vogls_assert_eq(x > 100, 1'b1);
    $vogls_assert_eq(x >>> 1, 64'h7FFF_FFFF_FFFF_FFFF);

    y = -1;
    $vogls_assert_eq(y, 32'hFFFF_FFFF);
    $vogls_assert_eq(y < 0, 1'b1);
    $vogls_assert_eq(y[31], 1'b1);

    y = -1; x = 1;
    $vogls_assert_eq(y > x, 1'b1);

    y = 2147483647;
    y = y + 1;
    $vogls_assert_eq(y, -2147483648);

    y = -8;
    $vogls_assert_eq(y >>> 1, -4);
    $vogls_assert_eq(y >> 1, 32'h7FFF_FFFC);
    x = -8;
    $vogls_assert_eq(x >>> 1, 64'h7FFF_FFFF_FFFF_FFFC);

    y = 32'bx;
    $vogls_assert_eq(y, 32'bx);
    x = 64'bz;
    $vogls_assert_eq(x, 64'bz);
    b = 64'hxxxx_xxxx_xxxx_xxxx;
    w = b;
    $vogls_assert_eq(w, 0.0);

    y = 2.5;
    $vogls_assert_eq(y, 3);
    y = 3.5;
    $vogls_assert_eq(y, 4);
    y = -2.5;
    $vogls_assert_eq(y, -3);
    y = 2.4;
    $vogls_assert_eq(y, 2);
    x = 2.5;
    $vogls_assert_eq(x, 64'd3);
    $vogls_assert_eq($rtoi(2.9), 2);
    $vogls_assert_eq($rtoi(-2.9), -2);
    $vogls_assert_eq($itor(3), 3.0);

    w = 7 / 2;
    $vogls_assert_eq(w, 3.0);
    w = 7.0 / 2;
    $vogls_assert_eq(w, 3.5);
    w = 0.5 + 0.25;
    $vogls_assert_eq(w, 0.75);

    w = 1.0;
    $vogls_assert_eq($realtobits(w), 64'h3FF0_0000_0000_0000);
    w = $bitstoreal(64'h4000_0000_0000_0000);
    $vogls_assert_eq(w, 2.0);

    z = 1.5;
    w = z;
    $vogls_assert_eq(w, 1.5);
    $vogls_assert_eq($realtobits(z), $realtobits(w));

    b = 64'hAAAA_5555_0000_00A5;
    x = b;
    $vogls_assert_eq(x[7:0], 8'hA5);
    $vogls_assert_eq(x[63], 1'b1);
    $vogls_assert_eq(x[63:32], 32'hAAAA_5555);

    #1.5;
    z = $realtime;
    $vogls_assert_eq($time, 64'd2);
    $vogls_assert_eq(z, 1.5);
    #0.4;
    $vogls_assert_eq($time, 64'd2);
    $vogls_assert_eq($realtime, 1.9);
  end
endmodule
