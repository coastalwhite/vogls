module tb();
    reg [7:0] a;
    assign a[3 -: 4] = 4'h2;
    assign a[7 -: 4] = 4'h4;
	initial #0 $vogls_assert_eq(a, 8'h42);
endmodule
