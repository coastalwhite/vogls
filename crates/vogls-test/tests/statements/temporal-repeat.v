// vogls: verify-stdout
module tb();
    reg clk;
    initial begin
        repeat (100) @(posedge clk);
        $display("%0t", $time());
        $finish();
    end
    always begin clk = 0; #5 clk = 1; #5 ; end
endmodule
