namespace ContentTags;

public class TextEscaper : IEscaper
{
    public TextEscaper()
    {
    }

    public static TextEscaper Instance
    {
        get
        {
            return ContentTagsGlobal.textEscaperInstance;
        }
    }

    public string ApplyString(string x)
    {
        return "[" + x + "]";
    }
}
