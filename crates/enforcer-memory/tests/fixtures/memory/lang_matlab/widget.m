classdef Widget
  methods
    function obj = draw(obj)
      if isempty(obj.name)
        obj = helper(obj);
      end
      close all
    end
  end
end

function out = helper(obj)
  out = disp(obj);
end
