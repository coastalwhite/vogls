module tb;
    reg [1:0] a;
    wire [1:0] rev;

    assign rev[0] = a[1];
    assign rev[1] = a[0];

    initial begin
		#0
        a = 2'b01;
        #1;
        $vogls_assert_eq(rev, 2'b10);
    end
endmodule
