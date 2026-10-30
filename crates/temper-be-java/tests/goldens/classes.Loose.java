package classes;

final class Loose {
    public final int n;

    public Loose(int n) {
        this.n = n;
    }

    public boolean eq(Loose other) {
        if (this.n == other.n) {
            return true;
        } else {
            return this.n == -other.n;
        }
    }

    public int getN() {
        return this.n;
    }
}
