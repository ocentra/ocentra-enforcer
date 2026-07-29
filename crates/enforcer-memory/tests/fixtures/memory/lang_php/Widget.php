<?php

namespace WidgetApp;

interface Drawable
{
    public function draw(): string;
}

class BaseWidget
{
    const MAX_WIDGETS = 10;

    public function describe(): string
    {
        return "base";
    }
}

class Widget extends BaseWidget implements Drawable
{
    public string $name;

    public function __construct(string $name)
    {
        $this->name = $name;
    }

    public function draw(): string
    {
        return $this->loadWidgetSettings($this->name);
    }

    private function loadWidgetSettings(string $name): string
    {
        return $name;
    }
}
