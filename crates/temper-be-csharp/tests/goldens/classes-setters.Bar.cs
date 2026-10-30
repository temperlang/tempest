namespace Setters;

internal class Bar
{
    private string backing;

    public Bar(string backing)
    {
        this.backing = backing;
    }

    public string Bar_
    {
        get
        {
            return this.backing;
        }
        private set
        {
            this.backing = value;
        }
    }

    public void UsingSet(string value)
    {
        this.Bar_ = value;
    }
}
