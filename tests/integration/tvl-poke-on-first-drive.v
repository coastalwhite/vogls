// vogls: verify-stdout
module top();
	reg a;
    always @(a) begin
        $display("hi!");
    end

    initial begin
        #0
        a = 0;
    end
endmodule
