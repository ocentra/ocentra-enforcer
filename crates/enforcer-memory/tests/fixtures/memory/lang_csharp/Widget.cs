using System;

namespace WidgetApp
{
    public interface IDrawable
    {
        string Draw();
    }

    public class BaseWidget
    {
        public const int MaxWidgets = 10;

        public virtual string Describe()
        {
            return "base";
        }
    }

    public class Widget : BaseWidget, IDrawable
    {
        public string Name;

        public Widget(string name)
        {
            Name = name;
        }

        public string Draw()
        {
            return LoadWidgetSettings(Name);
        }

        private string LoadWidgetSettings(string name)
        {
            return name;
        }
    }
}
