defmodule Widget do
  alias Helper.Text
  require Logger

  def draw(name) when name != "" do
    Logger.info("drawing")
    Text.upcase(name)
  end

  def draw(_name) do
    "unnamed"
  end

  defp helper(label) do
    label
  end
end
