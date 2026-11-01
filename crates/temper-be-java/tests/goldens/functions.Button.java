package functions;

import java.util.function.IntSupplier;

final class Button {
    public final String label;
    public final IntSupplier onPress;

    public Button(String label, IntSupplier onPress) {
        this.label = label;
        this.onPress = onPress;
    }

    public String press() {
        return this.label + " " + this.onPress.getAsInt();
    }

    public String getLabel() {
        return this.label;
    }

    public IntSupplier getOnPress() {
        return this.onPress;
    }

    public static final class Builder {
        String label;

        public Builder label(String label) {
            this.label = label;
            return this;
        }

        IntSupplier onPress;

        public Builder onPress(IntSupplier onPress) {
            this.onPress = onPress;
            return this;
        }

        public Button build() {
            if (label == null || onPress == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (label == null) {
                    _message.append(" label");
                }
                if (onPress == null) {
                    _message.append(" onPress");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Button(label, onPress);
        }
    }
}
