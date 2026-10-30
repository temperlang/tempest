package write_back_through_field;

final class Ping {
    public final Pong peer;
    public int n;
    public int most;

    public Ping(Pong peer) {
        this.peer = peer;
        this.n = 0;
        this.most = 0;
    }

    public int ping() {
        this.peer.pong(this);
        return this.n;
    }

    public int pass() {
        return this.peer.bump(this, this.n);
    }

    public boolean record_() {
        this.n = this.n + 1;
        if (this.n > this.most) {
            this.most = this.n;
        }
        return this.n > this.most - 1;
    }

    public int around() {
        return this.n + this.peer.bump(this, 10) + this.n;
    }

    public Pong getPeer() {
        return this.peer;
    }

    public int getN() {
        return this.n;
    }

    public void setN(int newN) {
        this.n = newN;
    }

    public int getMost() {
        return this.most;
    }

    public void setMost(int newMost) {
        this.most = newMost;
    }
}
