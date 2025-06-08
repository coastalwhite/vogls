module b(
    input clk
);
    always @ (posedge clk) $display("Hello!");

    initial begin
        clk <= 0;
        #4
        clk <= 1;
        #5
        clk <= 0;
        #7
        clk <= 1;
        #1
        $finish;
    end
endmodule

module a();
	b _b();
	initial $display("Hello from A");
endmodule