class App {
    void Main() {
        int x = 0;
        {
            {
                int b = 1;
            }
        }

        while (true) {
            if (x == 9) {
                x++;
            }
            else if (x == 10) {
                break;
            }
            x += 3;
        }

        for (int i = 0; i < 10; i++) {
            x += 2;
        }

        return x;
    }
}