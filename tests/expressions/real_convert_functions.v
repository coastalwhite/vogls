module x();
    initial begin
        $vogls_assert_eq($realtobits(1.5),  64'h3FF8000000000000);
        $vogls_assert_eq($realtobits(1.0),  64'h3FF0000000000000);
        $vogls_assert_eq($realtobits(0.0),  64'h0000000000000000);
        $vogls_assert_eq($realtobits(-2.0), 64'hC000000000000000);

        $vogls_assert_eq($bitstoreal(64'h3FF8000000000000), 1.5);
        $vogls_assert_eq($bitstoreal(64'h0000000000000000), 0.0);
        $vogls_assert_eq($bitstoreal(64'hC000000000000000), -2.0);
        $vogls_assert_eq($bitstoreal($realtobits(3.75)), 3.75);
        $vogls_assert_eq($realtobits($bitstoreal(64'h4008000000000000)), 64'h4008000000000000);

        $vogls_assert_eq($itor(5), 5.0);
        $vogls_assert_eq($itor(-3), -3.0);
        $vogls_assert_eq($itor(0), 0.0);
        $vogls_assert_eq($sqrt($itor(16)), 4.0);

        $vogls_assert_eq($rtoi(3.75), 3);
        $vogls_assert_eq($rtoi(-3.75), -3);
        $vogls_assert_eq($rtoi(2.25), 2);
        $vogls_assert_eq($rtoi(8.0), 8);
        $vogls_assert_eq($rtoi(0.5), 0);
        $vogls_assert_eq($rtoi(-0.5), 0);
        $vogls_assert_eq($rtoi(-2.999), -2);

        $vogls_assert_eq($rtoi($itor(7)), 7);
        $vogls_assert_eq($itor($rtoi(4.9)), 4.0);
    end
endmodule
