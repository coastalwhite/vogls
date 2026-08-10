module x();
    localparam [63:0] RB0 = $realtobits(1.5);
    localparam [63:0] RB1 = $realtobits(1.0);
    localparam [63:0] RB2 = $realtobits(0.0);
    localparam [63:0] RB3 = $realtobits(-2.0);

    localparam real BR0 = $bitstoreal(64'h3FF8000000000000);
    localparam real BR1 = $bitstoreal(64'h0000000000000000);
    localparam real BR2 = $bitstoreal(64'hC000000000000000);
    localparam real BR3 = $bitstoreal($realtobits(3.75));
    localparam [63:0] BR4 = $realtobits($bitstoreal(64'h4008000000000000));

    localparam real IT0 = $itor(5);
    localparam real IT1 = $itor(-3);
    localparam real IT2 = $itor(0);
    localparam real IT3 = $sqrt($itor(16));

    localparam integer RI0 = $rtoi(3.75);
    localparam integer RI1 = $rtoi(-3.75);
    localparam integer RI2 = $rtoi(3.5);
    localparam integer RI3 = $rtoi(2.25);
    localparam integer RI4 = $rtoi(8.0);
    localparam integer RI5 = $rtoi(0.5);
    localparam integer RI6 = $rtoi(-0.5);
    localparam integer RI7 = $rtoi(2.999);
    localparam integer RI8 = $rtoi($itor(7));
    localparam real    RI9 = $itor($rtoi(4.9));

    initial begin
        $vogls_assert_eq(RB0, 64'h3FF8000000000000);
        $vogls_assert_eq(RB1, 64'h3FF0000000000000);
        $vogls_assert_eq(RB2, 64'h0000000000000000);
        $vogls_assert_eq(RB3, 64'hC000000000000000);

        $vogls_assert_eq(BR0, 1.5);
        $vogls_assert_eq(BR1, 0.0);
        $vogls_assert_eq(BR2, -2.0);
        $vogls_assert_eq(BR3, 3.75);
        $vogls_assert_eq(BR4, 64'h4008000000000000);

        $vogls_assert_eq(IT0, 5.0);
        $vogls_assert_eq(IT1, -3.0);
        $vogls_assert_eq(IT2, 0.0);
        $vogls_assert_eq(IT3, 4.0);

        $vogls_assert_eq(RI0, 3);
        $vogls_assert_eq(RI1, -3);
        $vogls_assert_eq(RI2, 3);
        $vogls_assert_eq(RI3, 2);
        $vogls_assert_eq(RI4, 8);
        $vogls_assert_eq(RI5, 0);
        $vogls_assert_eq(RI6, 0);
        $vogls_assert_eq(RI7, 2);
        $vogls_assert_eq(RI8, 7);
        $vogls_assert_eq(RI9, 4.0);
    end
endmodule
