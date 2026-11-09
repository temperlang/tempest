namespace PrivateMethod;

internal class Note
{
    private readonly string body;

    public Note(string body)
    {
        this.body = body;
    }

    public string Body
    {
        get
        {
            return this.body;
        }
    }

    public Note Text(string more)
    {
        return new Note(this.body + more);
    }
}
