using System.Text;
using TemperLang.Core;

namespace Build;

public static class BuildGlobal
{
    static BuildGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Build");
        StringBuilder builder = new StringBuilder();
        builder.Append("Hi ");
        StringUtil.AppendCodePoint(builder, 127759);
        string source = "Wow!! Neato!";
        int bangIndex = StringUtil.Step(source, 0, 3);
        int spaceIndex = StringUtil.Step(source, bangIndex, 2);
        StringUtil.AppendBetween(builder, source, bangIndex, spaceIndex);
        string built = builder.ToString();
        builder.Append('\n');
        builder.Append(built);
        console.Log(builder.ToString());
        try
        {
            StringUtil.AppendCodePoint(builder, 1114112);
        }
        catch
        {
            console.Log("High code blocked.");
        }
        try
        {
            StringUtil.AppendCodePoint(builder, 55296);
        }
        catch
        {
            console.Log("Surrogate blocked.");
        }
        console.Log(builder.ToString());
        builder.Clear();
        builder.Append("Back to square zero.");
        console.Log(builder.ToString());
        StringBuilderOwner owner = new StringBuilderOwner(new StringBuilder());
        owner.Builder.Append("Hi from inside.");
        console.Log(owner.Builder.ToString());
    }
}
