#!/usr/bin/env ruby

require 'yaml'

BASE_URL = 'http://localhost:8000'.freeze
RUNS = 1000

CLI_RESULT_REGEX = /after (.*) turns\. (.*) is the winner/

def factorial(n)
  return 1 if n == 0

  (1..n).inject(:*) || 1
end

def combination(n, x)
  factorial(n) / (factorial(x) * factorial(n - x))
end

def binomial(n, x, p)
  combination(n, x) * (p**x) * ((1 - p)**(n - x))
end

def cumulative_probability(n, x, p)
  (x..n).sum { |i| binomial(n, i, p) }
end

class Snake
  attr_reader :name

  def initialize(name)
    @name = name
  end

  def url
    "#{BASE_URL}/#{name}"
  end
end

snake_names = %w[hovering-hobbs devious-devin]
snakes = snake_names.map { |n| Snake.new(n) }

snake_args = snakes.map { |s| "-n #{s.name} -u #{s.url}" }.join ' '

wins = {}
draws = []

def print_output(wins)
  total_runs = wins.values.flatten.count

  big_winner, winning_turns = wins.max_by { |_, v| v.count }
  winning_pct = winning_turns.count.to_f / total_runs * 100.0

  puts
  puts
  puts "The big winner is ... #{big_winner}! They won #{winning_pct}% of the rounds (out of #{total_runs} total non draw rounds)"
end

def print_binomial(snakes, wins, draws)
  total_runs = wins.values.sum(&:count) + draws.count

  first_snake, second_snake = snakes
  wins_for_first_snake = wins[first_snake.name]&.count || 0
  # wins_for_second_snake = wins[second_snake].count || 0

  binomial_prob = binomial(total_runs, wins_for_first_snake, 0.5)
  cummulative_prob = cumulative_probability(total_runs, wins_for_first_snake, 0.5)

  puts "The cumulative probability of #{first_snake.name} being better than #{second_snake.name} is #{1 - cummulative_prob}"
  puts "The binomial probability of this result is #{binomial_prob}"

  if binomial_prob < 0.001
    # First snake is better
    puts "We reached signifigance!"

    if cummulative_prob > 0.5
      puts "#{first_snake.name} is better than #{second_snake.name} with a score thingy of #{cummulative_prob}!"
    else
      puts "#{second_snake.name} is better than #{first_snake.name} with a score thingy of #{1 - cummulative_prob}!"
    end

    true
  else
    puts "We don't have enoiugh data to say anything"
    puts

    false
  end
end

trap('SIGINT') do
  print_output(wins)
  exit!
end

(0...RUNS).each do |i|
  run_result = `battlesnake play #{snake_args} -H 11 -W 11 -t 500 2>&1 >/dev/null | tail -n1`
  match = CLI_RESULT_REGEX.match(run_result)
  if match
    turns, winning_name = match.captures

    wins[winning_name] ||= []
    wins[winning_name] << turns

    puts "#{winning_name} won round #{i} in #{turns} turns"
    puts "Current Results: #{wins.map { |k, v| [k, v.count] }}"

    break if print_binomial(snakes, wins, draws)
  else
    draws << i
    puts "Round #{i} was a draw"
  end
end

print_output wins
# print_binomial(snakes, wins, draws)

# puts "FullResults\n#{wins.to_yaml}"
