// vogls: verify-stdout
module x();
    task delay(input value);
        #1 ;
    endtask

    initial begin
        $display("start");
        delay(0);
        $display("end");
    end
endmodule
