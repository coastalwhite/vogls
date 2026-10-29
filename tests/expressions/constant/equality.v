module t;
  localparam [71:0] W = 1;
  localparam        E = (W == 1);
  initial $vogls_assert_eq(E, 1);
endmodule
