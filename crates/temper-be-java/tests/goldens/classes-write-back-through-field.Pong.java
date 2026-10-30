package write_back_through_field;

final class Pong {
    public Pong() {}

    public void pong(Ping a) {
        {
            Ping o = a;
            o.n = o.n + 1;
        }
    }

    public int bump(Ping a, int by) {
        {
            Ping o = a;
            o.n = o.n + by;
        }
        return a.n;
    }
}
