module y(
    input a,
    output reg b
);
    initial b = 0;
    always #5 b = ~b;
endmodule

module x();
    wire b;
    y _y ( .b(b) );

    initial begin
        #0
        $vogls_assert_eq(b, 0);
        #6
        $vogls_assert_eq(b, 1);
        #5
        $vogls_assert_eq(b, 0);
        $finish();
    end
endmodule
