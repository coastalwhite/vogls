module x();
  reg [7:0]  mem1;
  reg [7:0]  mem2 [0:3];
  reg [15:0] mem3 [0:3][0:1];

  integer i, j;
  initial begin
    mem1 = 8'hA9;

    mem2[0] = 8'hAA;
    mem2[1] = 8'hBB;
    mem2[2] = 8'hCC;
    mem2[3] = 8'hDD;

    for(i = 0; i < 4; i = i + 1) begin
      for(j = 0; j < 2; j = j + 1) begin
        mem3[i][j] = i + j;
      end
    end

	$vogls_assert_eq(mem1, 8'hA9);

	$vogls_assert_eq(mem2[3], 8'hDD);
	$vogls_assert_eq(mem2[2], 8'hCC);
	$vogls_assert_eq(mem2[1], 8'hBB);
	$vogls_assert_eq(mem2[0], 8'hAA);

	$vogls_assert_eq(mem3[0][0], 16'h0000);
	$vogls_assert_eq(mem3[0][1], 16'h0001);
	$vogls_assert_eq(mem3[1][0], 16'h0001);
	// $vogls_assert_eq(mem3[1][1], 16'h0002);
	// $vogls_assert_eq(mem3[2][0], 16'h0002);
	// $vogls_assert_eq(mem3[2][1], 16'h0003);
	// $vogls_assert_eq(mem3[3][0], 16'h0003);
	// $vogls_assert_eq(mem3[3][1], 16'h0004);
  end
endmodule
