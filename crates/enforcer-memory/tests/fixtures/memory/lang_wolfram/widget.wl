Needs["WidgetLib`"]

helper[label_] := If[label != "", label, "unnamed"]

draw[label_] := helper[label]
