package async_turns;

final class Counter {
    public int n;

    public Counter() {
        this.n = 0;
    }

    public void bump(String tag, int times) {
        AsyncTurnsGlobal.console.log(tag + " start");
        {
            int i = 0;
            while (i < times) {
                int before = this.n;
                this.n = before + 1;
                i = i + 1;
            }
        }
        AsyncTurnsGlobal.console.log(tag + " done n=" + this.n);
    }

    public int getN() {
        return this.n;
    }

    public void setN(int newN) {
        this.n = newN;
    }
}
