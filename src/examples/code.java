class App {
    public static void Main() {
        int x = 0;
        while (true) {
            if (x == 9) {
                x++;
            }
            else if (x == 10) {
                break;
            }
            x += 3;
        }
        return x;
    }
}