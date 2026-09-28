module tb();
    initial $vogls_assert_eq('d0, 0);
    initial $vogls_assert_eq('d100, 100);
    initial $vogls_assert_eq('d4_294_967_295, 64'hFFFF_FFFF); // u32::MAX
    initial $vogls_assert_eq('d4_294_967_296, 64'h1_0000_0000); // u32::MAX + 1
endmodule