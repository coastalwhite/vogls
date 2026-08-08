// vogls: verify-stdout
module x();
    initial begin
        $display("%s", 32'hFAFB_FCFD);
        $display("it says: \"%s\"", $vogls_blackbox("blah blah"));
    end
endmodule
