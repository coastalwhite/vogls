// vogls: time=100000
// vogls: verify-stdout
module tb();
    localparam PERIOD = 100_000;
    initial begin
        $display("T=%0d", $time());
        #(PERIOD / 2);
        $display("T=%0d", $time());
        #(PERIOD / 4);
        $display("T=%0d", $time());
    end
endmodule