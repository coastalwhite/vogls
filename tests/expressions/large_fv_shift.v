module t;
  reg [131:0] a, s;
  initial begin
    a = 132'h1_2345_6789_abcd_ef01_2345_6789_abcd_ef01;
    s = 132'h8_2345_6789_abcd_ef01_2345_6789_abcd_ef01;
    $vogls_assert_eq(a >> 66, 132'h0000000000000000048d159e26af37bc0);
    $vogls_assert_eq(a << 66, 132'h48d159e26af37bc040000000000000000);
    $vogls_assert_eq($signed(s) >>> 66, 132'hffffffffffffffffe08d159e26af37bc0);
  end
endmodule
