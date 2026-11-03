package property_members;

final class Low1 implements High {
    public final int i;

    public Low1(int i) {
        this.i = i;
    }

    public String toString() {
        return "Low1(" + this.i + ")";
    }

    public boolean eq(High other) {
        if (other instanceof Low1) {
            return this.i == ((Low1) other).i;
        } else {
            return false;
        }
    }

    public int getI() {
        return this.i;
    }
}
