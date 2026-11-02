package defaults;

import temper.core.Nullable;

final class Greeter {
    public final String greeting;
    public final String punct;

    public Greeter(@Nullable String greeting, @Nullable String punct) {
        String greeting2 = greeting != null ? greeting : "Hello";
        String punct2 = punct != null ? punct : "!";
        this.greeting = greeting2;
        this.punct = punct2;
    }

    public Greeter(@Nullable String greeting) {
        this(greeting, null);
    }

    public Greeter() {
        this(null, null);
    }

    public String greet(@Nullable String name, @Nullable Integer times) {
        String name2 = name != null ? name : "world";
        int times2 = times != null ? times : 1;
        String out = "";
        {
            int i = 0;
            while (i < times2) {
                out = out + this.greeting + ", " + name2 + this.punct;
                i = i + 1;
            }
        }
        return out;
    }

    public String greet(@Nullable String name) {
        return greet(name, null);
    }

    public String greet() {
        return greet(null, null);
    }

    public static String shout(String text, @Nullable String mark) {
        String mark2 = mark != null ? mark : "!";
        return text + mark2 + mark2;
    }

    public static String shout(String text) {
        return shout(text, null);
    }

    public String getGreeting() {
        return this.greeting;
    }

    public String getPunct() {
        return this.punct;
    }

    public static final class Builder {
        @Nullable String greeting;

        public Builder greeting(@Nullable String greeting) {
            this.greeting = greeting;
            return this;
        }

        @Nullable String punct;

        public Builder punct(@Nullable String punct) {
            this.punct = punct;
            return this;
        }

        public Greeter build() {
            return new Greeter(greeting, punct);
        }
    }
}
