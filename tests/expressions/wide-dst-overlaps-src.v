module top;
  reg [149:0] r0;
  reg [64:0] r2;
  reg [149:0] r3;
  initial begin
    r0 = 150'h0ff78dd2e2aaf13ce5b4f2e6c8bc1d0c7a30da;
    r2 = 65'h1_7fde_c9ac_1369_bb3b;
    r3 = 150'h289865eb922114b16885ad8d051cfbec1933e3;
    $vogls_assert_eq((~r0) < (r0[r3[3:0] +: 8] ^ r2), 1'b0);
  end
endmodule
