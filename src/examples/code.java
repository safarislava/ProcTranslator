class App {
    int f(int x) {
        return x + 5;
    }

    void Main() {
        int x = 0;
        int b = 1;


        if (x > b) {
            if (x + 1 > b) {
            
            }
            else {
            
            }
        }
        else if (x < b) {

        }
        else {

        }

        while (true) {
            if (x == 9) {
                x++;
            }
            else if (x == 10) {
                break;
            }
            x = this.f(x + 1);
        }

        for (int i = 0; i < 10; i++) {
            x += 2;
        }

        return;
    }
}