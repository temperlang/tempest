package libraries;

import temper.core.Nullable;

public final class Dot implements Shape {
    public final int size;

    public Dot(int size) {
        this.size = size;
    }

    public @Nullable Dot getBigger() {
        if (this.size < 3) {
            return new Dot(this.size + 1);
        } else {
            return null;
        }
    }

    public @Nullable Integer getGrown() {
        {
            @Nullable Dot subject = this.getBigger();
            if (subject == null) {
                return null;
            } else {
                return subject.scaled(10);
            }
        }
    }

    public int scaled(int k) {
        return this.size * k;
    }

    public int getSize() {
        return this.size;
    }
}
