// vogls: verify-stdout
// vogls: time=2000
module y #( parameter SHOW = 1, parameter D = 42 ) ();
    if (SHOW) begin
        initial #D $display("Printed %0h", D);
    end
endmodule

module x();
    y #( .SHOW(0), .D(13)   ) y1 ();
    y #( .SHOW(1), .D(42)   ) y2 ();
    y #( .SHOW(1), .D(1337) ) y3 ();
    y #( .SHOW(0), .D(37)   ) y4 ();
endmodule
