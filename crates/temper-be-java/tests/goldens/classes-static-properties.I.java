package static_properties;

interface I {
    static void sayBye() {
        StaticPropertiesGlobal.console.log("I says 'bye.'");
    }
}
