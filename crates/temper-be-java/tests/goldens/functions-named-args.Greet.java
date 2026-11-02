package named_args;

import temper.core.Nullable;

public final class Greet {
    public Greet(@Nullable String message, @Nullable Boolean repeat) {
        String message2 = message != null ? message : "Hi!";
        boolean repeat2 = repeat != null ? repeat : false;
        NamedArgsGlobal.console.log(message2);
        if (repeat2) {
            NamedArgsGlobal.console.log(message2);
        }
    }

    public Greet(@Nullable String message) {
        this(message, null);
    }

    public Greet() {
        this(null, null);
    }

    public static final class Builder {
        @Nullable String message;

        public Builder message(@Nullable String message) {
            this.message = message;
            return this;
        }

        @Nullable Boolean repeat;

        public Builder repeat(@Nullable Boolean repeat) {
            this.repeat = repeat;
            return this;
        }

        public Greet build() {
            return new Greet(message, repeat);
        }
    }
}
