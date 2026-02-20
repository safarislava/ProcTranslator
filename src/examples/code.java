class App {
    int f(int x) {
        return x + 5;
    }

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
            x = f(x + 1);
        }

        for (int i = 0; i < 10; i++) {
            x += 2;
        }

        return;
    }
}