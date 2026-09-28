// vogls: verify-stdout
module x();
    initial begin
        $display("in module: %m");
        begin: y
            $display("in named block: %m");
        end
        z(1);
    end
    task z(input f);
        $display("in task: %m");
    endtask
endmodule
