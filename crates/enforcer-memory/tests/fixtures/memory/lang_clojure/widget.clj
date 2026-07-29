(ns app.widget
  (:require [clojure.string :as str]))

(defrecord Widget [name])

(defn helper [x y]
  (+ x y))

(defn draw [widget]
  (if (> (helper 1 2) 0)
    (println (str/join " " ["drawing" (:name widget)]))
    (println "drawing: nothing"))
  (helper 3 4))

(defn main []
  (draw (Widget. "box")))
